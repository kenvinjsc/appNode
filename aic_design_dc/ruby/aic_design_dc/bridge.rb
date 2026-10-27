# encoding: UTF-8
# Gọi lõi C++ qua Fiddle và dựng hình vào model (đơn vị mm).
require 'fiddle'
require 'json'
require_relative 'params'

module AicDesignDC
  module Core
    LIB_NAME = RUBY_PLATFORM =~ /mswin|mingw/ ? 'hsx_core.dll' : (RUBY_PLATFORM =~ /darwin/ ? 'libhsx_core.dylib' : 'libhsx_core.so')
    LIB = Fiddle.dlopen(File.join(__dir__, LIB_NAME))
    SOLVE = Fiddle::Function.new(LIB['hsx_solve'], [Fiddle::TYPE_VOIDP, Fiddle::TYPE_VOIDP, Fiddle::TYPE_INT], Fiddle::TYPE_INT)
    DXF = Fiddle::Function.new(LIB['hsx_export_dxf'], [Fiddle::TYPE_VOIDP, Fiddle::TYPE_VOIDP], Fiddle::TYPE_INT)

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
  end

  COLORS = { 'carcass' => [235, 232, 224], 'back' => [200, 190, 170],
             'front' => [248, 248, 246], 'drawer' => [215, 200, 175] }.freeze

  module Builder
    def self.material(model, role)
      m = model.materials["hsx_#{role}"] || model.materials.add("hsx_#{role}")
      m.color = Sketchup::Color.new(*COLORS.fetch(role, [220, 220, 220]))
      m
    end

    def self.add_panel(ents, pn, model)
      g = ents.add_group
      g.name = pn['name']
      o = pn['o'].map(&:mm)
      s = pn['s'].map(&:mm)
      pts = [[0, 0], [s[0], 0], [s[0], s[1]], [0, s[1]]].map { |x, y| Geom::Point3d.new(o[0] + x, o[1] + y, o[2]) }
      face = g.entities.add_face(pts)
      face.reverse! if face.normal.z < 0
      face.pushpull(s[2])
      g.material = material(model, pn['role'])
      g.set_attribute('hsx', 'role', pn['role'])
      g.set_attribute('hsx', 'holes', JSON.generate(pn['holes']))
      g
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
      params[:cols] = rescale_cols(old, params) if changes.keys.map(&:to_sym).include?(:width) && !changes.key?(:cols) && !changes.key?('cols')
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
