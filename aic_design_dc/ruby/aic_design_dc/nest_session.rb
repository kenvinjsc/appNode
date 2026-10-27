# encoding: UTF-8
require 'json'
require_relative 'bridge'

module AicDesignDC
  # Gom chi tiết toàn model + xếp ván tối ưu, KHÔNG làm đơ giao diện SketchUp:
  #  1) Quét model theo từng lát ngắn (~6 ms) bằng UI.start_timer — nhả luồng UI giữa các lát.
  #     Chỉ đọc thuộc tính tham số của từng tủ/chi tiết; không tạo hình, không chèn gì vào model.
  #  2) Phần tính nặng (giải từng tủ, gom chi tiết, MaxRects) chạy ở LUỒNG NỀN của lõi C++.
  #  3) Ruby chỉ thăm dò trạng thái mỗi ~120 ms (đọc nguyên tử) và lấy kết quả khi xong.
  module NestJob
    SLICE_SECONDS = 0.006   # thời gian tối đa mỗi lát quét
    TICK = 0.01             # nghỉ giữa các lát để UI vẽ lại
    POLL = 0.12             # chu kỳ thăm dò luồng nền
    MAX_DEPTH = 8

    DEFAULTS = { sheet_l: 2440, sheet_w: 1220, kerf: 4, trim: 10, rotate: false }.freeze

    class Session
      attr_reader :handle

      # on_progress.call(phase, value): phase :scan (value = số đối tượng đã quét) | :compute (value 0..1)
      def initialize(model, opts, selection_only: false, on_progress: nil, on_done: nil, on_error: nil)
        @model = model
        @opts = DEFAULTS.merge((opts || {}).transform_keys(&:to_sym))
        @selection_only = selection_only
        @on_progress = on_progress
        @on_done = on_done
        @on_error = on_error
        @counts = Hash.new(0)
        @scanned = 0
        @cancelled = false
        @handle = nil
        @timer = nil
      end

      def start
        roots = @selection_only ? @model.selection.to_a : @model.active_entities.to_a
        @stack = [[roots, 0, 0]]
        schedule_scan
        self
      end

      def cancel
        @cancelled = true
        stop_timer
        Core.nest_cancel(@handle) if @handle
      end

      def dispose
        cancel
        if @handle
          Core.nest_free(@handle)
          @handle = nil
        end
      end

      def sheet(mat, idx)
        @handle ? Core.nest_sheet(@handle, mat, idx) : nil
      end

      def export_dxf(path)
        @handle ? Core.nest_export_dxf(@handle, path) : false
      end

      private

      def stop_timer
        UI.stop_timer(@timer) if @timer
        @timer = nil
      end

      def schedule_scan
        @timer = UI.start_timer(TICK, false) { scan_slice }
      end

      def fail(msg)
        stop_timer
        @on_error&.call(msg)
      end

      # Một lát quét: duyệt cây thực thể không đệ quy, dừng khi hết SLICE_SECONDS.
      def scan_slice
        return if @cancelled
        t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        begin
          until @stack.empty?
            list, i, depth = @stack.last
            if i >= list.size
              @stack.pop
            else
              @stack.last[1] = i + 1
              visit(list[i], depth)
              @scanned += 1
            end
            break if Process.clock_gettime(Process::CLOCK_MONOTONIC) - t0 > SLICE_SECONDS
          end
        rescue StandardError => e
          return fail("Lỗi khi quét model: #{e.message}")
        end
        @on_progress&.call(:scan, @scanned)
        @stack.empty? ? begin_compute : schedule_scan
      end

      def visit(ent, depth)
        return unless ent.valid?
        return unless ent.is_a?(Sketchup::Group) || ent.is_a?(Sketchup::ComponentInstance)
        return if ent.hidden? || (ent.layer && !ent.layer.visible?)
        prm = ent.get_attribute('hsx', 'params')
        return (@counts[[:cab, prm]] += 1) if prm
        part = ent.get_attribute('hsx', 'part_params')
        return (@counts[[:part, part]] += 1) if part
        return if depth >= MAX_DEPTH
        sub = ent.is_a?(Sketchup::Group) ? ent.entities : ent.definition.entities
        @stack << [sub.to_a, 0, depth + 1]
      end

      def job_text
        o = @opts
        head = ["sheet_l=#{o[:sheet_l]}", "sheet_w=#{o[:sheet_w]}", "kerf=#{o[:kerf]}", "trim=#{o[:trim]}", "rotate=#{o[:rotate] ? 1 : 0}"]
        secs = @counts.map do |(kind, json), n|
          prm = kind == :cab ? Params.sanitize(JSON.parse(json)) : PartParams.sanitize(JSON.parse(json))
          "count=#{n}\n#{Core.encode(prm)}"
        end
        ([head.join("\n")] + secs).join("\n---\n")
      end

      def begin_compute
        return fail('Không tìm thấy tủ/chi tiết nào do AicDesignDC tạo' + (@selection_only ? ' trong vùng chọn' : '')) if @counts.empty?
        @handle = Core.nest_start(job_text)           # trả về ngay; việc nặng chạy ở luồng nền
        @timer = UI.start_timer(POLL, true) { poll }
      rescue StandardError => e
        fail("Không bắt đầu được xếp ván: #{e.message}")
      end

      def poll
        return if @cancelled
        st, pr = Core.nest_status(@handle)
        @on_progress&.call(:compute, pr)
        return if st == 0
        stop_timer
        res = Core.nest_result(@handle)
        if st == 1 && res && res['ok']
          @on_done&.call(res)
        else
          @on_error&.call(st == 3 ? 'Đã hủy' : (res && res['error']) || 'Xếp ván thất bại')
        end
      rescue StandardError => e
        fail("Lỗi khi lấy kết quả: #{e.message}")
      end
    end

    # CSV (UTF-8 có BOM để Excel đọc đúng tiếng Việt) từ kết quả gom chi tiết.
    def self.csv(summary)
      fmt = ->(v) { v.to_f.zero? ? '-' : format('%.2f', v) }
      rows = [%w[Vật_liệu Tên Dài_cắt Rộng_cắt Số_lượng Dán_cạnh_dài Dán_cạnh_ngắn].map { |h| h.tr('_', ' ') }]
      summary['items'].sort_by { |i| [i['mat'], -i['w'], -i['h']] }.each do |i|
        b = i['band']
        rows << [i['mat'], i['name'], i['w'], i['h'], i['qty'], "#{fmt.(b[0])}/#{fmt.(b[1])}", "#{fmt.(b[2])}/#{fmt.(b[3])}"]
      end
      "﻿" + rows.map { |r| r.map { |c| c.to_s.include?(';') ? "\"#{c}\"" : c.to_s }.join(';') }.join("\r\n") + "\r\n"
    end
  end
end
