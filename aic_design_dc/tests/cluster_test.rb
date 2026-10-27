# encoding: UTF-8
# Kiểm tra logic cụm tủ (không cần SketchUp; cần thư viện C++ cạnh bridge.rb). Chạy: ruby -Ku tests/cluster_test.rb
class Numeric; def mm; self * 0.0393701; end; end
module Sketchup; class Group; end; end
dir = File.expand_path('../ruby/aic_design_dc', __dir__)
%w[params bridge library cluster].each { |f| require File.join(dir, f) }
include AicDesignDC
abort Core::LOAD_ERROR unless Core.available?
def check(c, m); abort "FAIL: #{m}" unless c; end

sp = Cluster.sample_kitchen
Cluster.placements(sp).each { |pl| r = Core.solve(pl[:params]); check(r['ok'], "#{pl[:row]['name']}: #{r['error']}") }
check(Cluster.total_width(sp) == 2950.0, 'tổng rộng')
mo = Cluster.mouldings(sp)
check(mo.size == 1 && mo[0][:z] == 2375.0 && mo[0][:len] == 2950.0, 'phào chạy suốt tủ treo')

big = Cluster.resize(sp, 3500.0 / 2950, 1, 1)
check((Cluster.total_width(big) - 3500).abs < 0.11, 'giãn rộng')
big['rows'].each { |r| check((Cluster.row_width(r) - 3500).abs < 0.11, 'mỗi hàng đủ rộng') }
Cluster.placements(big).each { |pl| check(Core.solve(pl[:params])['ok'], 'khối sau giãn dựng được') }

fx = Marshal.load(Marshal.dump(sp)); fx['rows'][0]['modules'][1]['fixed'] = true
r2 = Cluster.resize(fx, 3500.0 / 2950, 1.1, 0.9)
check(r2['rows'][0]['modules'][1]['w'] == 900.0, 'khối fixed giữ rộng')
check(r2['rows'][0]['modules'][0]['params']['depth'] == (561 * 1.1).round(1), 'sâu theo ys')
check(r2['rows'][1]['modules'][0]['params']['hang_z'] == (1495 * 0.9).round(1), 'treo cao theo zs')
check(sp['rows'][0]['modules'][0]['w'] == 400, 'spec gốc không bị sửa')

begin; Cluster.resize(sp, 0.2, 1, 1); check(false, 'phải báo quá hẹp'); rescue ArgumentError => e; check(e.message.include?('quá hẹp'), 'thông báo quá hẹp'); end

cl = Cluster.cutlist(sp)
check(cl.sum { |r| r['qty'] } == Cluster.placements(sp).sum { |pl| Core.solve(pl[:params])['panels'].size }, 'bảng cắt cụm khớp số tấm')
puts 'CLUSTER OK'
