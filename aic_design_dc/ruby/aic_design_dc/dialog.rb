# encoding: UTF-8
require 'json'
require_relative 'bridge'
require_relative 'draw_tool'
require_relative 'part_tool'
require_relative 'library'
require_relative 'nest_session'

module AicDesignDC
  module Dialog
    @dlg = nil
    @nest = nil

    def self.selected_cab
      sel = Sketchup.active_model.selection
      sel.length == 1 && Builder.cabinet?(sel.first) ? sel.first : nil
    end

    # Đẩy tham số của tủ đang chọn (hoặc mặc định) lên hộp thoại.
    def self.sync
      return unless @dlg && @dlg.visible?
      cab = selected_cab
      h = cab ? Builder.params_of(cab) : State.params
      @dlg.execute_script("setParams(#{JSON.generate(JSON.generate(h)).to_s},#{cab ? 'true' : 'false'})")
    end

    def self.push(js)
      @dlg.execute_script(js) if @dlg && @dlg.visible?
    end

    def self.push_library
      push("setLibrary(#{JSON.generate(JSON.generate(Library.all))})")
    end

    def self.push_part
      push("setPart(#{JSON.generate(JSON.generate(State.part))})")
      res = Core.solve(State.part)
      push("setPartInfo(#{JSON.generate(JSON.generate('ok' => res['ok'], 'error' => res['error'], 'info' => res['info']))})")
    end

    # Nạp mẫu từ thư viện.
    def self.load_template(name)
      e = Library.find(name)
      raise "Không thấy mẫu: #{name}" unless e
      if e['kind'] == 'part'
        State.part = PartParams.sanitize(e['params'])
        push_part
      else
        cab = selected_cab
        merged = Params.sanitize(e['params'], Params::DEFAULTS)  # mẫu tính từ mặc định
        if cab
          new_cab = Builder.rebuild(Sketchup.active_model, cab, merged)
          Sketchup.active_model.selection.clear
          Sketchup.active_model.selection.add(new_cab)
        else
          State.params = merged
        end
        sync
      end
    end

    # Gom chi tiết + xếp ván: quét theo lát nhỏ, tính ở luồng nền (xem nest_session.rb).
    def self.nest_start(opts)
      @nest&.dispose
      @last_summary = nil
      scope_sel = opts.delete('scope') == 'selection'
      o = { sheet_l: opts['sheet_l'].to_f, sheet_w: opts['sheet_w'].to_f, kerf: opts['kerf'].to_f, trim: opts['trim'].to_f, rotate: opts['rotate'] ? true : false }
      @nest = NestJob::Session.new(
        Sketchup.active_model, o, selection_only: scope_sel,
        on_progress: ->(phase, v) { push("nestProgress(#{JSON.generate(phase.to_s)},#{v.to_f})") },
        on_done: ->(res) { @last_summary = res; push("nestDone(#{JSON.generate(JSON.generate(res))})") },
        on_error: ->(msg) { push("nestError(#{JSON.generate(msg.to_s)})") }
      ).start
    end

    def self.apply(json)
      incoming = JSON.parse(json)
      model = Sketchup.active_model
      cab = selected_cab
      if cab
        old = Builder.params_of(cab)
        changes = Params.sanitize(incoming, old)
        changes.delete(:cols) if changes[:cols] == old[:cols]  # chỉ đổi rộng -> để lõi co giãn khoang cố định
        new_cab = Builder.rebuild(model, cab, changes)
        model.selection.clear
        model.selection.add(new_cab)
      else
        State.params = Params.sanitize(incoming, State.params)
      end
    rescue StandardError => e
      UI.messagebox("AicDesignDC: #{e.message}")
      sync
    end

    def self.show
      if @dlg && @dlg.visible?
        @dlg.bring_to_front
        return
      end
      @dlg = UI::HtmlDialog.new(dialog_title: 'Thông số', preferences_key: 'AicDesignDC.Params',
                                scrollable: true, resizable: true, width: 340, height: 860, left: 60, top: 80, style: UI::HtmlDialog::STYLE_DIALOG)
      @dlg.set_file(File.join(__dir__, 'dialog.html'))
      @dlg.add_action_callback('ready') { |_c| push_library; push_part; sync }
      @dlg.add_action_callback('part_apply') do |_c, json|
        State.part = PartParams.sanitize(JSON.parse(json))
        res = Core.solve(State.part)  # báo ngay thông số có hợp lệ không (vd. mộng quá dày, hèm quá sâu)
        push("setPartInfo(#{JSON.generate(JSON.generate('ok' => res['ok'], 'error' => res['error'], 'info' => res['info']))})")
      end
      @dlg.add_action_callback('place_part') { |_c| Sketchup.active_model.select_tool(PlaceTool.new) }
      @dlg.add_action_callback('lib_load') do |_c, name|
        begin
          load_template(name)
        rescue StandardError => e
          UI.messagebox("AicDesignDC: #{e.message}")
        end
      end
      @dlg.add_action_callback('lib_save') do |_c, json|
        begin
          e = JSON.parse(json)
          params = e['kind'] == 'part' ? PartParams.sanitize(e['params']) : Params.sanitize(e['params'])
          Library.save(e['name'], e['kind'], params)
          push_library
        rescue StandardError => err
          UI.messagebox("AicDesignDC: #{err.message}")
        end
      end
      @dlg.add_action_callback('lib_delete') do |_c, name|
        Library.delete(name)
        push_library
      end
      @dlg.add_action_callback('nest_start') { |_c, json| nest_start(JSON.parse(json)) }
      @dlg.add_action_callback('nest_cancel') { |_c| @nest&.cancel }
      @dlg.add_action_callback('nest_sheet') do |_c, mat, idx|
        sh = @nest&.sheet(mat.to_i, idx.to_i)
        push("nestSheet(#{JSON.generate(JSON.generate(sh))})") if sh
      end
      @dlg.add_action_callback('nest_dxf') do |_c|
        path = UI.savepanel('Xuất DXF xếp ván', nil, 'xep_van.dxf')
        UI.messagebox(@nest&.export_dxf(path) ? "Đã xuất: #{path}" : 'Xuất DXF thất bại') if path
      end
      @dlg.add_action_callback('nest_csv') do |_c|
        path = UI.savepanel('Xuất bảng chi tiết gộp (CSV)', nil, 'chi_tiet_gop.csv')
        if path && @last_summary
          File.write(path, NestJob.csv(@last_summary), encoding: 'UTF-8')
          UI.messagebox("Đã xuất: #{path}")
        end
      end
      @dlg.add_action_callback('apply') { |_c, json| apply(json) }
      @dlg.add_action_callback('draw') { |_c| Sketchup.active_model.select_tool(DrawTool.new) }
      @dlg.add_action_callback('cutlist') do |_c|
        cab = selected_cab
        rows = cab ? Builder.cutlist(cab) : Core.solve(State.params)['cutlist']
        @dlg.execute_script("showCutlist(#{JSON.generate(JSON.generate(rows))})")
      end
      @dlg.add_action_callback('dxf') do |_c|
        cab = selected_cab
        path = UI.savepanel('Xuất DXF', nil, 'tu.dxf')
        if path
          ok = Core.export_dxf(cab ? Builder.params_of(cab) : State.params, path)
          UI.messagebox(ok ? "Đã xuất: #{path}" : 'Xuất DXF thất bại')
        end
      end
      @dlg.show
    end
  end
end
