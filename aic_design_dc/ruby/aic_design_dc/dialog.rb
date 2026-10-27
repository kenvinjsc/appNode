# encoding: UTF-8
require 'json'
require_relative 'bridge'
require_relative 'draw_tool'
require_relative 'part_tool'
require_relative 'library'

module AicDesignDC
  module Dialog
    @dlg = nil

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
      @dlg.add_action_callback('part_apply') { |_c, json| State.part = PartParams.sanitize(JSON.parse(json)) }
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
