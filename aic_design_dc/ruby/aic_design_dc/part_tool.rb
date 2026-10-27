# encoding: UTF-8
require_relative 'bridge'

module AicDesignDC
  # Đặt chi tiết lẻ (uốn cong, đợt bo góc, phào, xà): click 1 điểm. Xem trước hộp bao.
  class PlaceTool
    def activate
      @ip = Sketchup::InputPoint.new
      @pos = nil
      res = Core.solve(PartParams.sanitize(State.part))
      @box = res['ok'] ? res['panels'].map { |p| [p['o'], p['s']] } : []
      Sketchup.status_text = 'Click để đặt chi tiết. Esc: thoát'
      Sketchup.active_model.active_view.invalidate
    end

    def deactivate(view)
      view.invalidate
    end

    def onMouseMove(_f, x, y, view)
      @ip.pick(view, x, y)
      @pos = @ip.position
      view.tooltip = @ip.tooltip
      view.invalidate
    end

    def onLButtonDown(_f, x, y, view)
      @ip.pick(view, x, y)
      model = Sketchup.active_model
      begin
        g = Builder.build_part(model, State.part, Geom::Transformation.new(@ip.position))
        model.selection.clear
        model.selection.add(g)
      rescue StandardError => e
        UI.messagebox("AicDesignDC: #{e.message}")
      end
      view.invalidate
    end

    def onCancel(_reason, _view)
      Sketchup.active_model.select_tool(nil)
    end

    def draw(view)
      return unless @pos
      view.drawing_color = 'blue'
      view.line_width = 1
      @box.each do |o, s|
        c = [0, 1].product([0, 1], [0, 1]).map do |i, j, k|
          Geom::Point3d.new(@pos.x + (o[0] + i * s[0]).mm, @pos.y + (o[1] + j * s[1]).mm, @pos.z + (o[2] + k * s[2]).mm)
        end
        [[0, 1], [2, 3], [4, 5], [6, 7], [0, 2], [1, 3], [4, 6], [5, 7], [0, 4], [1, 5], [2, 6], [3, 7]].each do |a, b|
          view.draw(GL_LINES, [c[a], c[b]])
        end
      end
    end

    def getExtents
      bb = Geom::BoundingBox.new
      bb.add(@pos) if @pos
      bb
    end
  end
end
