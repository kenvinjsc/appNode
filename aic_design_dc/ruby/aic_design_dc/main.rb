# encoding: UTF-8
require 'sketchup.rb'
require_relative 'params'
require_relative 'bridge'
require_relative 'draw_tool'
require_relative 'dialog'
require_relative 'observers'

module AicDesignDC
  unless @loaded
    @loaded = true
    cmd_draw = UI::Command.new('Vẽ tủ') { Sketchup.active_model.select_tool(DrawTool.new) }
    cmd_draw.tooltip = 'Vẽ tủ tham số (click 2 điểm)'
    cmd_dlg = UI::Command.new('Thông số') { Dialog.show }
    cmd_dlg.tooltip = 'Thông số khung tủ'

    menu = UI.menu('Extensions').add_submenu('AicDesignDC')
    menu.add_item(cmd_draw)
    menu.add_item(cmd_dlg)

    tb = UI::Toolbar.new('AicDesignDC')
    tb.add_item(cmd_draw)
    tb.add_item(cmd_dlg)
    tb.show

    Observers.install
  end
end
