# encoding: UTF-8
require_relative 'bridge'

module AicDesignDC
  # Bắt thao tác Scale gốc của SketchUp trên tủ -> tính lại tủ theo lõi C++.
  module Observers
    @busy = false

    def self.normalize_all(model)
      return if @busy
      @busy = true
      begin
        model.active_entities.grep(Sketchup::Group).select { |g| Builder.cabinet?(g) && Builder.scaled?(g) }.each do |g|
          begin
            n = Builder.normalize_scale(model, g)
            model.selection.clear
            model.selection.add(n)
          rescue StandardError => e
            UI.messagebox("AicDesignDC: #{e.message}")
          end
        end
      ensure
        @busy = false
      end
    end

    def self.defer(model)
      UI.start_timer(0, false) { normalize_all(model) }  # không sửa model trong callback của observer
    end

    class Tools < Sketchup::ToolsObserver
      def onToolStateChanged(tools, tool_name, _id, state)
        Observers.defer(Sketchup.active_model) if tool_name.to_s =~ /scale/i && state.zero?
      end

      def onActiveToolChanged(tools, _name, _id)
        Observers.defer(Sketchup.active_model)
      end
    end

    class Sel < Sketchup::SelectionObserver
      def onSelectionBulkChange(_s)
        Dialog.sync
      end
      def onSelectionCleared(_s)
        Dialog.sync
      end
    end

    class App < Sketchup::AppObserver
      def onNewModel(m)
        Observers.attach(m)
      end
      def onOpenModel(m)
        Observers.attach(m)
      end
    end

    def self.attach(model)
      model.tools.add_observer(Tools.new)
      model.selection.add_observer(Sel.new)
    end

    def self.install
      attach(Sketchup.active_model)
      Sketchup.add_observer(App.new)
    end
  end
end
