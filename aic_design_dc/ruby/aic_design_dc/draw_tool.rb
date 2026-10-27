# encoding: UTF-8
require_relative 'bridge'
require_relative 'library'

module AicDesignDC
  # Vẽ khung tủ: Tab đổi mẫu khung (rỗng) sẵn có. Click điểm 1, di chuột/click điểm 2 (hoặc gõ "rộng,sâu" rồi Enter).
  class DrawTool
    MIN = 100 # mm

    TAB_KEY = 9

    # Các mẫu khung rỗng để chọn nhanh bằng Tab (mục đầu = thông số mặc định hiện tại).
    def templates
      @templates ||= [nil] + Library.all.select { |e| e['kind'] == 'cabinet' && e['name'].to_s.start_with?('Khung') }
    end

    def params_now
      e = templates[@tpl.to_i]
      e ? Params.sanitize(e['params'], DEFAULTS) : State.params
    end

    def onKeyDown(key, _repeat, _flags, view)
      return unless key == TAB_KEY
      @tpl = (@tpl.to_i + 1) % templates.length
      status
      view.invalidate
    end

    def activate
      @tpl = 0
      @p1 = nil
      @p2 = nil
      @ip = Sketchup::InputPoint.new
      Sketchup.active_model.active_view.invalidate
      status
    end

    def deactivate(view)
      view.invalidate
    end
    def resume(view)
      status
      view.invalidate
    end
    def onCancel(_reason, view)
      @p1 = @p2 = nil
      status
      view.invalidate
    end

    def status
      name = templates[@tpl.to_i] ? templates[@tpl.to_i]['name'] : 'thông số hiện tại'
      if @p1
        Sketchup.status_text = "Khung: #{name} | click điểm 2 (hoặc gõ rộng,sâu + Enter) | Tab: đổi mẫu | Esc: hủy"
        Sketchup.vcb_label = 'Rộng, Sâu'
      else
        Sketchup.status_text = "Khung: #{name} | click điểm 1 (góc trái-trước) | Tab: đổi mẫu khung"
        Sketchup.vcb_label = ''
      end
    end

    def onMouseMove(_flags, x, y, view)
      @ip.pick(view, x, y)
      if @p1
        @p2 = @ip.position
        Sketchup.vcb_value = "#{dims[0].round}, #{dims[1].round}"
      end
      view.tooltip = @ip.tooltip
      view.invalidate
    end

    def onLButtonDown(_flags, x, y, view)
      @ip.pick(view, x, y)
      if @p1.nil?
        @p1 = @ip.position
        @p2 = @p1
        status
      else
        @p2 = @ip.position
        finish(*dims)
      end
      view.invalidate
    end

    def onUserText(text, _view)
      return unless @p1
      w, d = text.split(/[,;]/).map { |t| t.strip.to_l.to_mm }
      sx = @p2 && @p2.x < @p1.x ? -1 : 1
      sy = @p2 && @p2.y < @p1.y ? -1 : 1
      @p2 = Geom::Point3d.new(@p1.x + sx * w.mm, @p1.y + sy * (d || params_now[:depth]).mm, @p1.z)
      finish(*dims)
    rescue ArgumentError
      UI.beep
    end

    def dims
      return [0, 0] unless @p1 && @p2
      w = (@p2.x - @p1.x).abs.to_mm.round
      d = (@p2.y - @p1.y).abs.to_mm.round
      [w, d]
    end

    def finish(w, d)
      return UI.beep if w < MIN
      d = params_now[:depth] if d < 50 # kéo thẳng 1 phía -> dùng chiều sâu mặc định
      origin = Geom::Point3d.new([@p1.x, @p2.x].min, [@p1.y, @p2.y].min, @p1.z)
      model = Sketchup.active_model
      begin
        cab = Builder.build(model, params_now.merge(width: w, depth: d), Geom::Transformation.new(origin))
        model.selection.clear
        model.selection.add(cab)
      rescue StandardError => e
        UI.messagebox("AicDesignDC: #{e.message}")
      end
      @p1 = @p2 = nil
      status
    end

    # Xem trước khung bao tủ
    def draw(view)
      return unless @p1 && @p2
      w, d = dims
      return if w < 1
      h = params_now[:height].mm
      x0 = [@p1.x, @p2.x].min
      y0 = [@p1.y, @p2.y].min
      z0 = @p1.z
      x1 = x0 + w.mm
      y1 = y0 + (d < 50 ? params_now[:depth].mm : d.mm)
      c = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
      bot = c.map { |x, y| Geom::Point3d.new(x, y, z0) }
      top = c.map { |x, y| Geom::Point3d.new(x, y, z0 + h) }
      view.drawing_color = 'blue'
      view.line_width = 2
      view.draw(GL_LINE_LOOP, bot)
      view.draw(GL_LINE_LOOP, top)
      4.times { |i| view.draw(GL_LINES, [bot[i], top[i]]) }
    end

    def getExtents
      bb = Geom::BoundingBox.new
      [@p1, @p2].compact.each { |p| bb.add(p) }
      bb.add(@p1.offset(Z_AXIS, params_now[:height].mm)) if @p1
      bb
    end
  end
end
