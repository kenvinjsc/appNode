# encoding: UTF-8
require 'json'
require_relative 'params'

module AicDesignDC
  # Thanh chip nổi của công cụ (như trong video): Đợt/Vách, kiểu cánh, số bản lề...
  module OptionBar
    HTML = <<~'HTML'.freeze
      <!doctype html><html><head><meta charset="utf-8"><style>
       body{margin:0;padding:5px 8px;font:12px "Segoe UI",sans-serif;background:#f4f4f2;color:#222;user-select:none;overflow:hidden}
       #root{display:flex;flex-wrap:nowrap;gap:10px;align-items:center;white-space:nowrap}
       .g{display:flex;gap:3px;align-items:center}
       .lbl{color:#777;font-size:11px}
       button{border:1px solid #c9c9c4;background:#fff;border-radius:10px;padding:1px 8px;font-size:12px;cursor:pointer}
       button.on{background:#2a7be4;border-color:#2a7be4;color:#fff}
      </style></head><body><div id="root"></div><script>
       var S={}, KIND='';
       var DEF={
        divide:[{l:'Chia',k:'mode',o:[['shelf','Đợt'],['divider','Vách']]}],
        drawer:[{l:'Ngăn',k:'drawer_count',o:[[2,'2'],[3,'3'],[4,'4'],[5,'5']]},{l:'Cao',k:'drawer_pitch',o:[[150,'150'],[180,'180'],[200,'200'],[250,'250']]},{l:'',k:'clear',o:[[0,'Đặt'],[1,'Xóa']]}],
        door:[{l:'Cánh',k:'door_style',o:[[1,'Trùm ngoài'],[2,'Trùm nửa'],[3,'Lọt trong']]},{l:'Bản lề',k:'hinges',o:[[0,'Tự'],[2,'2'],[3,'3'],[4,'4'],[5,'5']]},{l:'Số cánh',k:'door_count',o:[[-1,'Tự'],[1,'1'],[2,'2'],[3,'3']]},{l:'',k:'clear',o:[[0,'Đặt'],[1,'Xóa']]}]
       };
       function render(){
        var d=DEF[KIND]; if(!d) return;
        document.getElementById('root').innerHTML=d.map(function(r){
         return '<div class="g">'+(r.l?'<span class="lbl">'+r.l+'</span>':'')+r.o.map(function(o){
          var on=String(S[r.k])===String(o[0]); return '<button class="'+(on?'on':'')+'" onclick="pick(\''+r.k+'\','+JSON.stringify(o[0]).replace(/"/g,'&quot;')+')">'+o[1]+'</button>';}).join('')+'</div>';}).join('');
       }
       function pick(k,v){S[k]=v; render(); sketchup.chip(k,String(v));}
       function setState(kind,s){KIND=kind;S=JSON.parse(s);render();}
      </script></body></html>
    HTML

    @dlg = nil

    def self.show(kind)
      close
      @dlg = UI::HtmlDialog.new(dialog_title: 'AicDesignDC – tuỳ chọn công cụ', preferences_key: 'AicDesignDC.Bar',
                                scrollable: false, resizable: false, width: kind == 'door' ? 560 : 440, height: 62, left: 80, top: 110,
                                style: UI::HtmlDialog::STYLE_UTILITY)
      @dlg.set_html(HTML)
      @dlg.add_action_callback('chip') do |_c, key, value|
        apply(key, value)
      end
      @dlg.show
      UI.start_timer(0.15, false) { push(kind) }   # chờ HTML nạp xong rồi đẩy trạng thái hiện tại
    end

    def self.push(kind)
      @dlg&.execute_script("setState(#{JSON.generate(kind)}, #{JSON.generate(JSON.generate(State.tool))})")
    end

    def self.apply(key, value)
      k = key.to_sym
      v = k == :mode ? value.to_s : value.to_i
      return unless State.tool.key?(k) || k == :clear
      State.tool[k] = v
    end

    def self.close
      @dlg&.close
    rescue StandardError
      nil
    ensure
      @dlg = nil
    end
  end
end
