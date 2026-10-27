# encoding: UTF-8
require 'sketchup.rb'
require_relative 'params'
require_relative 'bridge'
require_relative 'draw_tool'
require_relative 'part_tool'
require_relative 'library'
require_relative 'dialog'
require_relative 'opening_tools'
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

    cmd_div = UI::Command.new('Đợt / Vách') { Sketchup.active_model.select_tool(DivideTool.new) }
    cmd_div.tooltip = 'Chia khoang bằng đợt ngang / vách đứng (Alt đổi; gõ 3, /3, 200)'
    cmd_drw = UI::Command.new('Ngăn kéo') { Sketchup.active_model.select_tool(DrawerTool.new) }
    cmd_drw.tooltip = 'Đặt ngăn kéo vào khoang'
    cmd_door = UI::Command.new('Cánh') { Sketchup.active_model.select_tool(DoorTool.new) }
    cmd_door.tooltip = 'Đặt cánh (trùm ngoài / trùm nửa / lọt trong) vào khoang'

    icons = File.join(__dir__, 'icons')
    { cmd_draw => 'draw', cmd_dlg => 'params', cmd_part => 'part', cmd_div => 'divide', cmd_drw => 'drawer', cmd_door => 'door' }.each do |cmd, name|
      cmd.small_icon = File.join(icons, "#{name}.svg")
      cmd.large_icon = File.join(icons, "#{name}.svg")
    end

    menu = UI.menu('Extensions').add_submenu('AicDesignDC')
    menu.add_item(cmd_draw)
    menu.add_item(cmd_dlg)
    menu.add_item(cmd_div)
    menu.add_item(cmd_drw)
    menu.add_item(cmd_door)
    menu.add_item(cmd_part)

    tb = UI::Toolbar.new('AicDesignDC')
    tb.add_item(cmd_draw)
    tb.add_item(cmd_dlg)
    tb.add_item(cmd_div)
    tb.add_item(cmd_drw)
    tb.add_item(cmd_door)
    tb.add_item(cmd_part)
    tb.show

    Observers.install
  end
end
