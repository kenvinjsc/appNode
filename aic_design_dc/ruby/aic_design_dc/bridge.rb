# encoding: UTF-8
# Gọi lõi C++ qua Fiddle và dựng hình vào model (đơn vị mm).
require 'fiddle'
require 'json'
require_relative 'params'

module AicDesignDC
  module Core
    LIB_NAME = RUBY_PLATFORM =~ /mswin|mingw/ ? 'hsx_core.dll' : (RUBY_PLATFORM =~ /darwin/ ? 'libhsx_core.dylib' : 'libhsx_core.so')
    LOAD_ERROR = nil
    begin
      LIB = Fiddle.dlopen(File.join(__dir__, LIB_NAME))
      SOLVE = Fiddle::Function.new(LIB['hsx_solve'], [Fiddle::TYPE_VOIDP, Fiddle::TYPE_VOIDP, Fiddle::TYPE_INT], Fiddle::TYPE_INT)
      DXF = Fiddle::Function.new(LIB['hsx_export_dxf'], [Fiddle::TYPE_VOIDP, Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT)
      # xếp ván chạy ở luồng nền của lõi C++: Ruby chỉ start / thăm dò / lấy kết quả (đều rất nhẹ)
      N_START = Fiddle::Function.new(LIB['hsx_nest_start'], [Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT)
      N_STATUS = Fiddle::Function.new(LIB['hsx_nest_status'], [Fiddle::TYPE_INT, Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT)
      N_CANCEL = Fiddle::Function.new(LIB['hsx_nest_cancel'], [Fiddle::TYPE_INT], Fiddle::TYPE_VOID)
      N_RESULT = Fiddle::Function.new(LIB['hsx_nest_result'], [Fiddle::TYPE_INT, Fiddle::TYPE_VOIDP, Fiddle::TYPE_INT], Fiddle::TYPE_INT)
      N_SHEET = Fiddle::Function.new(LIB['hsx_nest_sheet'], [Fiddle::TYPE_INT, Fiddle::TYPE_INT, Fiddle::TYPE_INT, Fiddle::TYPE_VOIDP, Fiddle::TYPE_INT], Fiddle::TYPE_INT)
      N_DXF = Fiddle::Function.new(LIB['hsx_nest_export_dxf'], [Fiddle::TYPE_INT, Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT)
      N_FREE = Fiddle::Function.new(LIB['hsx_nest_free'], [Fiddle::TYPE_INT], Fiddle::TYPE_VOID)
    rescue StandardError, LoadError => e
      remove_const(:LOAD_ERROR)
      const_set(:LOAD_ERROR, "Không nạp được #{LIB_NAME}: #{e.message}")
    end

    # Thiếu/hỏng DLL thì plugin báo lỗi rõ ràng thay vì làm hỏng lúc nạp (bài học từ WPS).
    def self.available?
      LOAD_ERROR.nil?
    end

    def self.encode(params)
      lines = params.reject { |k, _| k == :cols }.map { |k, v| "#{k}=#{v == true ? 1 : v == false ? 0 : v}" }
      (params[:cols] || []).each { |c| lines << "col=#{c.join(',')}" }
      lines.join("\n")
    end

    def self.solve(params)
      src = encode(params)
      n = SOLVE.call(src, nil, 0)
      buf = Fiddle::Pointer.malloc(n + 1)
      SOLVE.call(src, buf, n + 1)
      JSON.parse(buf.to_s(n))
    end

    def self.export_dxf(params, path)
      DXF.call(encode(params), path) == 1
    end

    # --- xếp ván nền ---
    def self.nest_start(job)
      h = N_START.call(job)
      raise 'Không tạo được luồng xếp ván' if h <= 0
      h
    end

    # => [trạng thái(0 chạy,1 xong,2 lỗi,3 hủy,-1 sai handle), tiến độ 0..1]
    def self.nest_status(h)
      buf = Fiddle::Pointer.malloc(8)
      st = N_STATUS.call(h, buf)
      [st, buf[0, 8].unpack1('E')]
    end

    def self.nest_cancel(h)
      N_CANCEL.call(h)
    end

    def self.read_json(fn, *args)
      n = fn.call(*args, nil, 0)
      return nil if n < 0
      buf = Fiddle::Pointer.malloc(n + 1)
      fn.call(*args, buf, n + 1)
      JSON.parse(buf.to_s(n))
    end

    def self.nest_result(h)
      read_json(N_RESULT, h)
    end

    def self.nest_sheet(h, mat, sheet)
      read_json(N_SHEET, h, mat, sheet)
    end

    def self.nest_export_dxf(h, path)
      N_DXF.call(h, path) == 1
    end

    def self.nest_free(h)
      N_FREE.call(h)
    end
  end

  COLORS = { 'carcass' => [235, 232, 224], 'back' => [200, 190, 170],
             'front' => [248, 248, 246], 'drawer' => [215, 200, 175],
             'bend' => [235, 232, 224], 'moulding' => [244, 240, 230], 'glass' => [190, 225, 235] }.freeze

  PART_NAMES = { 'door_frame' => 'Cánh khung' }.freeze

  module Builder
    def self.material(model, role)
      m = model.materials["hsx_#{role}"] || model.materials.add("hsx_#{role}")
      m.color = Sketchup::Color.new(*COLORS.fetch(role, [220, 220, 220]))
      m
    end

    # Làm mịn các cạnh của mặt cong (góc giữa 2 mặt kề nhỏ) để hiển thị cung tròn mượt.
    def self.smooth_curved(ents)
      limit = 35.degrees
      ents.grep(Sketchup::Edge).each do |ed|
        next unless ed.faces.size == 2
        next unless ed.faces[0].normal.angle_between(ed.faces[1].normal) < limit
        ed.soft = true
        ed.smooth = true
      end
    end

    # Khoét/phay trên mặt tấm: mỗi pocket là 1 đa giác trên mặt top/bottom, đẩy vào trong `depth`.
    def self.cut_pockets(g, pn, o, s)
      ax = pn['axis']
      ia = (ax + 1) % 3
      ib = (ax + 2) % 3
      dir = [X_AXIS, Y_AXIS, Z_AXIS][ax]
      (pn['pockets'] || []).each do |pk|
        c = pk['top'] ? o[ax] + s[ax] : o[ax]
        pts = pk['loop'].each_slice(2).map do |u, v|
          a = [0, 0, 0]
          a[ax] = c
          a[ia] = u.mm
          a[ib] = v.mm
          Geom::Point3d.new(*a)
        end
        face = g.entities.add_face(pts)
        next unless face
        outward = pk['top'] ? dir : dir.reverse
        dist = pk['depth'].mm
        face.pushpull(face.normal.dot(outward) > 0 ? -dist : dist)
      end
    end

    def self.add_panel(ents, pn, model)
      g = ents.add_group
      g.name = pn['name']
      o = pn['o'].map(&:mm)
      s = pn['s'].map(&:mm)
      if pn['poly'] && pn['axis']
        ax = pn['axis']
        ia = (ax + 1) % 3
        ib = (ax + 2) % 3
        pts = pn['poly'].each_slice(2).map do |u, v|
          c = [0, 0, 0]
          c[ax] = o[ax]
          c[ia] = u.mm
          c[ib] = v.mm
          Geom::Point3d.new(*c)
        end
        face = g.entities.add_face(pts)
        dir = [X_AXIS, Y_AXIS, Z_AXIS][ax]
        face.reverse! if face.normal.dot(dir) < 0
        face.pushpull(s[ax])
        cut_pockets(g, pn, o, s)
        smooth_curved(g.entities)
      else
        pts = [[0, 0], [s[0], 0], [s[0], s[1]], [0, s[1]]].map { |x, y| Geom::Point3d.new(o[0] + x, o[1] + y, o[2]) }
        face = g.entities.add_face(pts)
        face.reverse! if face.normal.z < 0
        face.pushpull(s[2])
      end
      g.material = material(model, pn['role'])
      g.set_attribute('hsx', 'role', pn['role'])
      g.set_attribute('hsx', 'holes', JSON.generate(pn['holes']))
      g
    end

    # Chi tiết lẻ (uốn cong, đợt bo góc, phào, xà): 1 group, lưu tham số để sửa/xuất sau.
    def self.build_part(model, params, transformation = nil)
      params = PartParams.sanitize(params)
      res = Core.solve(params)
      raise res['error'].to_s unless res['ok']
      model.start_operation('AicDesignDC: Chi tiết', true)
      begin
        grp = model.active_entities.add_group
        grp.name = res['panels'].size > 1 ? PART_NAMES.fetch(params[:kind], 'Chi tiết') : res['panels'].first['name']
        Sketchup.status_text = res['info'] unless res['info'].to_s.empty?
        res['panels'].each { |pn| add_panel(grp.entities, pn, model) }
        grp.transformation = transformation if transformation
        grp.set_attribute('hsx', 'part_params', JSON.generate(params))
        model.commit_operation
        grp
      rescue StandardError
        model.abort_operation
        raise
      end
    end

    def self.cabinet?(e)
      e.is_a?(Sketchup::Group) && !e.get_attribute('hsx', 'params').nil?
    end

    def self.params_of(cab)
      Params.sanitize(JSON.parse(cab.get_attribute('hsx', 'params')))
    end

    def self.build(model, params, transformation = nil, operation: true)
      params = Params.sanitize(params)
      res = Core.solve(params)
      raise res['error'].to_s unless res['ok']
      model.start_operation('AicDesignDC: Tủ', true) if operation
      cab = model.active_entities.add_group
      cab.name = 'Khung tủ'
      res['panels'].each { |pn| add_panel(cab.entities, pn, model) }
      cab.transformation = transformation if transformation
      cab.set_attribute('hsx', 'params', JSON.generate(params))
      model.commit_operation if operation
      cab
    rescue StandardError
      model.abort_operation if operation
      raise
    end

    # Khi đổi bề rộng: co giãn tỉ lệ các khoang có bề rộng cố định để tổng vẫn khớp.
    def self.rescale_cols(old, new_params)
      return new_params[:cols] if old[:width] == new_params[:width]
      n = old[:cols].size
      oi = old[:width] - 2 * old[:thick] - (n - 1) * old[:thick]
      ni = new_params[:width] - 2 * new_params[:thick] - (n - 1) * new_params[:thick]
      k = ni / oi
      new_params[:cols].map { |c| c[0] > 0 ? [(c[0] * k).round(1)] + c[1..-1] : c }
    end

    # Tính lại toàn bộ từ tham số rồi dựng lại cùng vị trí (không kéo méo hình học).
    def self.rebuild(model, cab, changes, transformation = nil, transparent: false)
      old = params_of(cab)
      params = Params.sanitize(changes, old)
      legacy = old[:layout].to_s.empty?
      params[:cols] = rescale_cols(old, params) if legacy && changes.keys.map(&:to_sym).include?(:width) && !changes.key?(:cols) && !changes.key?('cols')
      tr = transformation || cab.transformation
      model.start_operation('AicDesignDC: Sửa tủ', true, false, transparent)
      begin
        res = Core.solve(params)
        raise res['error'].to_s unless res['ok']  # lỗi thì giữ nguyên tủ cũ
        cab.erase!
        new_cab = build(model, params, tr, operation: false)
        model.commit_operation
        new_cab
      rescue StandardError
        model.abort_operation
        raise
      end
    end

    # --- Kéo giãn bằng công cụ Scale gốc của SketchUp ---
    def self.scaled?(cab)
      tr = cab.transformation
      [tr.xscale, tr.yscale, tr.zscale].any? { |v| (v - 1).abs > 1e-4 }
    end

    # Đưa scale về 1, đổi kích thước tham số tương ứng, dựng lại: ván/kính giữ độ dày, ngăn kéo giữ chiều cao.
    def self.normalize_scale(model, cab)
      tr = cab.transformation
      p = params_of(cab)
      changes = { width: (p[:width] * tr.xscale).round(1), depth: (p[:depth] * tr.yscale).round(1),
                  height: (p[:height] * tr.zscale).round(1) }
      rigid = Geom::Transformation.axes(tr.origin, tr.xaxis.normalize, tr.yaxis.normalize, tr.zaxis.normalize)
      rebuild(model, cab, changes, rigid, transparent: true)
    end

    def self.cutlist(cab)
      Core.solve(params_of(cab))['cutlist']
    end
  end
end
