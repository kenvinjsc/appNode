# encoding: UTF-8
require 'sketchup.rb'
require_relative 'params'
require_relative 'bridge'
require_relative 'draw_tool'
require_relative 'part_tool'
require_relative 'library'
require_relative 'dialog'
require_relative 'observers'

module AicDesignDC
  if !Core.available?
    UI.messagebox("AicDesignDC: #{Core::LOAD_ERROR}\nHãy đặt #{Core::LIB_NAME} cạnh các file .rb của plugin (xem README).")
  elsif !@loaded
    @loaded = true
    cmd_draw = UI::Command.new('Vẽ tủ') { Sketchup.active_model.select_tool(DrawTool.new) }
    cmd_draw.tooltip = 'Vẽ tủ tham số (click 2 điểm)'
    cmd_dlg = UI::Command.new('Thông số') { Dialog.show }
    cmd_dlg.tooltip = 'Thông số, thư viện mẫu, chi tiết'
    cmd_part = UI::Command.new('Đặt chi tiết') { Sketchup.active_model.select_tool(PlaceTool.new) }
    cmd_part.tooltip = 'Đặt chi tiết uốn cong / đợt bo góc / phào / xà'

    icons = File.join(__dir__, 'icons')
    { cmd_draw => 'draw', cmd_dlg => 'params', cmd_part => 'part' }.each do |cmd, name|
      cmd.small_icon = File.join(icons, "#{name}.svg")
      cmd.large_icon = File.join(icons, "#{name}.svg")
    end

    menu = UI.menu('Extensions').add_submenu('AicDesignDC')
    menu.add_item(cmd_draw)
    menu.add_item(cmd_dlg)
    menu.add_item(cmd_part)

    tb = UI::Toolbar.new('AicDesignDC')
    tb.add_item(cmd_draw)
    tb.add_item(cmd_dlg)
    tb.add_item(cmd_part)
    tb.show

    Observers.install
  end
end
