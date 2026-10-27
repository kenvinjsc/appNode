# encoding: UTF-8
require 'json'
require_relative 'bridge'
require_relative 'layout'
require_relative 'option_bar'

module AicDesignDC
  # Công cụ làm việc trên KHOANG của khung tủ (quy trình như video): bấm vào khoang -> chia đợt/vách,
  # đặt ngăn kéo, đặt cánh. Mỗi thao tác sửa cây bố cục rồi dựng lại tủ từ lõi C++ (một bước Undo).
  class OpeningTool
    KIND = 'divide'.freeze
    HIGHLIGHT = Sketchup::Color.new(255, 170, 0, 70)
    EDGE = Sketchup::Color.new(230, 130, 0)

    def activate
      @cab = nil
      @op = nil
      @cache = {}
      OptionBar.show(self.class::KIND)
      status
      Sketchup.active_model.active_view.invalidate
    end

    def deactivate(view)
      OptionBar.close
      view.invalidate
    end

    def resume(view)
      status
      view.invalidate
    end

    def onCancel(_reason, view)
      @op = nil
      view.invalidate
    end

    def getExtents
      bb = Geom::BoundingBox.new
      bb.add(@cab.bounds) if @cab&.valid?
      bb
    end

    # ---- chọn tủ + khoang dưới con trỏ ----------------------------------------------------
    def cabinet_at(view, x, y)
      sel = Sketchup.active_model.selection
      return sel.first if sel.length == 1 && Builder.cabinet?(sel.first)
      ph = view.pick_helper
      ph.do_pick(x, y)
      path = ph.path_at(0) || []
      path.find { |e| Builder.cabinet?(e) }
    end

    def openings_of(cab)
      key = [cab.entityID, cab.get_attribute('hsx', 'params')]
      @cache.clear unless @cache.key?(key)
      @cache[key] ||= Core.solve(Builder.params_of(cab))['openings'] || []
    end

    def local_hit(cab, view, x, y)
      ray = view.pickray(x, y)
      inv = cab.transformation.inverse
      o = ray[0].transform(inv)
      d = ray[1].transform(inv)
      hit = Pick.front_plane_hit(o.to_a, d.to_a)
      hit && hit.map(&:to_mm)
    end

    def onMouseMove(_flags, x, y, view)
      @cab = cabinet_at(view, x, y)
      @op = nil
      if @cab
        @hit = local_hit(@cab, view, x, y)
        @op = Pick.opening_at(openings_of(@cab), @hit[0], @hit[1]) if @hit
      end
      if @op
        view.tooltip = format('%s  %d × %d mm', @op['t'], (@op['x1'] - @op['x0']).round, (@op['z1'] - @op['z0']).round)
        hover(@op, @hit)
      end
      view.invalidate
    end

    def hover(_op, _hit); end

    # Phím số bấm là áp dụng (như video: "gõ 1..9"); gõ nhiều số liền nhau thì gom lại, hết 0,6 s hoặc Enter mới áp dụng.
    DIGIT_KEYS = ((48..57).to_a + (96..105).to_a).freeze
    TAB_KEY = 9

    def digit_of(key)
      return nil unless DIGIT_KEYS.include?(key)
      key >= 96 ? key - 96 : key - 48
    end

    def onKeyDown(key, _repeat, _flags, view)
      if (d = digit_of(key))
        @buf = "#{@buf}#{d}"
        UI.stop_timer(@buf_timer) if @buf_timer
        @buf_timer = UI.start_timer(0.6, false) { flush_buffer(view) }
        return
      end
      key_down(key, view)
    end

    def key_down(_key, _view); end

    def flush_buffer(view)
      text = @buf.to_s
      @buf = nil
      @buf_timer = nil
      return if text.empty?
      @applied = [text, Time.now]
      handle_digits(text, view)
    end

    def onUserText(text, view)
      t = text.to_s.strip
      return if @applied && @applied[0] == t && Time.now - @applied[1] < 4   # đã áp dụng bằng phím số
      UI.stop_timer(@buf_timer) if @buf_timer
      @buf = @buf_timer = nil
      handle_text(t, view)
    end

    def handle_digits(text, view)
      handle_text(text, view)
    end

    def handle_text(_text, _view); end

    # ---- dựng lại tủ với bố cục mới --------------------------------------------------------
    def current_root(cab)
      Layout.from_params(Builder.params_of(cab))
    end

    def commit(root)
      model = Sketchup.active_model
      new_cab = Builder.rebuild(model, @cab, { layout: Layout.dump(root) })
      model.selection.clear
      model.selection.add(new_cab)
      @cab = new_cab
      @cache.clear
      @op = nil
      true
    rescue StandardError => e
      Sketchup.status_text = "AicDesignDC: #{e.message}"
      UI.beep
      false
    end

    # ---- vẽ ----------------------------------------------------------------------------------
    def world(pt_mm)
      @cab.transformation * Geom::Point3d.new(pt_mm[0].mm, pt_mm[1].mm, pt_mm[2].mm)
    end

    def draw_rect(view, o, color = HIGHLIGHT)
      return unless @cab&.valid? && o
      quad = [[o['x0'], 0, o['z0']], [o['x1'], 0, o['z0']], [o['x1'], 0, o['z1']], [o['x0'], 0, o['z1']]].map { |p| world(p) }
      view.drawing_color = color
      view.draw(GL_QUADS, quad)
      view.drawing_color = EDGE
      view.line_width = 2
      view.draw(GL_LINE_LOOP, quad)
    end

    def draw(view)
      draw_rect(view, @op) if @op
    end
  end

  # ---- Chia đợt / vách ---------------------------------------------------------------------
  class DivideTool < OpeningTool
    KIND = 'divide'.freeze

    def status
      Sketchup.status_text = 'Click: chia tại con trỏ | phím 2..9: chia N khoang | gõ 200 + Enter: cách mép 200 | Alt/Tab: đợt ↔ vách'
      Sketchup.vcb_label = 'Chia'
    end

    def mode
      State.tool[:mode] == 'divider' ? :divider : :shelf
    end

    def extent_cursor(op, hit)
      mode == :shelf ? [op['z1'] - op['z0'], hit[1] - op['z0']] : [op['x1'] - op['x0'], hit[0] - op['x0']]
    end

    def hover(op, hit)
      ext, cur = extent_cursor(op, hit)
      d = [cur, ext - cur].min
      Sketchup.vcb_value = format('%d mm', d.round)
    end

    def key_down(key, view)
      return unless key == ALT_MODIFIER_KEY || key == TAB_KEY
      State.tool[:mode] = mode == :shelf ? 'divider' : 'shelf'
      OptionBar.push('divide')
      view.invalidate
    end

    # một phím số (2..9) = chia khoang thành N khoang đều; nhiều chữ số = khoảng cách mm
    def handle_digits(text, view)
      handle_text(text.length == 1 ? "/#{text}" : text, view)
    end

    # xem trước tấm ván sẽ chia (ở vị trí con trỏ)
    def draw(view)
      return unless @op && @hit
      draw_rect(view, @op)
      t = State.params[:thick]
      ext, cur = extent_cursor(@op, @hit)
      sizes = Layout.divide_at(ext, cur, t)
      lo = sizes[0] + (mode == :shelf ? @op['z0'] : @op['x0'])
      pts = if mode == :shelf
              [[@op['x0'], 0, lo], [@op['x1'], 0, lo], [@op['x1'], 0, lo + t], [@op['x0'], 0, lo + t]]
            else
              [[lo, 0, @op['z0']], [lo + t, 0, @op['z0']], [lo + t, 0, @op['z1']], [lo, 0, @op['z1']]]
            end.map { |p| world(p) }
      view.drawing_color = Sketchup::Color.new(255, 220, 60, 200)
      view.draw(GL_QUADS, pts)
    end

    def onLButtonDown(_flags, x, y, view)
      onMouseMove(0, x, y, view)
      return unless @cab && @op && @hit
      ext, cur = extent_cursor(@op, @hit)
      divide(Layout.divide_at(ext, cur, State.params[:thick]))
      view.invalidate
    end

    def handle_text(text, view)
      return UI.beep unless @cab && @op && @hit
      ext, cur = extent_cursor(@op, @hit)
      sizes = Layout.parse_divide(text, ext, cur, State.params[:thick])
      return (Sketchup.status_text = 'Giá trị không hợp lệ: gõ 3, /3 hoặc khoảng cách mm'; UI.beep) unless sizes
      divide(sizes)
      view.invalidate
    end

    def divide(sizes)
      root = current_root(@cab)
      dir = mode == :shelf ? 'H' : 'V'
      commit(Layout.divide(root, @op['p'], dir, sizes))
    end
  end

  # ---- Ngăn kéo ----------------------------------------------------------------------------
  class DrawerTool < OpeningTool
    KIND = 'drawer'.freeze

    def status
      Sketchup.status_text = 'Click khoang: đặt ngăn kéo | phím 1..9: số ngăn | Tab: đặt ↔ xóa'
      Sketchup.vcb_label = 'Số ngăn'
      Sketchup.vcb_value = State.tool[:drawer_count].to_s
    end

    def key_down(key, view)
      return unless key == TAB_KEY
      State.tool[:clear] = State.tool[:clear].to_i == 1 ? 0 : 1
      OptionBar.push('drawer')
    end

    def handle_text(text, _view)
      n = text.to_i
      return UI.beep unless n >= 1 && n <= 12
      State.tool[:drawer_count] = n
      OptionBar.push('drawer')
      Sketchup.vcb_value = n.to_s
    end

    def onLButtonDown(_flags, x, y, view)
      onMouseMove(0, x, y, view)
      return unless @cab && @op
      clear = State.tool[:clear].to_i == 1
      count = clear ? 0 : State.tool[:drawer_count]
      pitch = State.tool[:drawer_pitch].to_f
      need = count * pitch + State.params[:thick]
      if count.positive? && need > (@op['z1'] - @op['z0'])
        Sketchup.status_text = format('Khoang cao %d mm, không đủ chỗ cho %d ngăn × %d mm', (@op['z1'] - @op['z0']).round, count, pitch)
        return UI.beep
      end
      begin
        commit(Layout.set_leaf(current_root(@cab), @op['p'], 'd' => count, 'p' => count.zero? ? 0 : pitch))
      rescue ArgumentError => e
        Sketchup.status_text = "AicDesignDC: #{e.message}"
        UI.beep
      end
      view.invalidate
    end
  end

  # ---- Cánh ---------------------------------------------------------------------------------
  class DoorTool < OpeningTool
    KIND = 'door'.freeze

    def status
      Sketchup.status_text = 'Click khoang: đặt cánh | phím 1..4: số cánh | Tab: kiểu cánh | Shift: cài đặt (dày cánh, khe, rộng tối đa)'
      Sketchup.vcb_label = 'Số cánh'
      Sketchup.vcb_value = State.tool[:door_count] < 0 ? 'tự' : State.tool[:door_count].to_s
    end

    def handle_text(text, _view)
      n = text.strip.downcase == 'a' ? -1 : text.to_i
      return UI.beep unless n == -1 || (n >= 1 && n <= 8)
      State.tool[:door_count] = n
      OptionBar.push('door')
    end

    def key_down(key, _view)
      if key == CONSTRAIN_MODIFIER_KEY
        settings
      elsif key == TAB_KEY   # Tab: đổi kiểu cánh trùm ngoài -> trùm nửa -> lọt trong
        State.tool[:door_style] = State.tool[:door_style] % 3 + 1
        OptionBar.push('door')
      end
    end

    def settings
      p = State.params
      res = UI.inputbox(['Dày cánh (mm)', 'Khe hở (mm)', 'Rộng tối đa một cánh (mm)'], [p[:door_thick], p[:gap], p[:door_max_w]], 'AicDesignDC: cài đặt cánh')
      return unless res
      State.params = Params.sanitize({ door_thick: res[0], gap: res[1], door_max_w: res[2] }, p)
      return unless @cab&.valid?
      model = Sketchup.active_model
      new_cab = Builder.rebuild(model, @cab, { door_thick: res[0], gap: res[1], door_max_w: res[2] })
      model.selection.clear
      model.selection.add(new_cab)
      @cab = new_cab
      @cache.clear
    rescue StandardError => e
      UI.messagebox("AicDesignDC: #{e.message}")
    end

    def onLButtonDown(_flags, x, y, view)
      onMouseMove(0, x, y, view)
      return unless @cab && @op
      clear = State.tool[:clear].to_i == 1
      attrs = if clear
                { 'l' => 0, 'm' => nil, 'h' => 0 }
              else
                { 'l' => State.tool[:door_count], 'm' => State.tool[:door_style], 'h' => State.tool[:hinges] }
              end
      begin
        commit(Layout.set_leaf(current_root(@cab), @op['p'], attrs))
      rescue ArgumentError => e
        Sketchup.status_text = "AicDesignDC: #{e.message}"
        UI.beep
      end
      view.invalidate
    end
  end
end
