# encoding: UTF-8
# Kiểm tra migration/sanitize (không cần SketchUp). Chạy: ruby -Ku tests/params_test.rb
class Numeric; def mm; self * 0.0393701; end; end
dir = File.expand_path('../ruby/aic_design_dc', __dir__)
%w[params bridge].each { |f| require File.join(dir, f) }
include AicDesignDC
abort Core::LOAD_ERROR unless Core.available?
def check(c, m); abort "FAIL: #{m}" unless c; end

old = { 'width' => 1200, 'height' => 2000, 'cols' => [[0, 2, 0, 0, 0], [0, 0, 0, 2, 2]] }   # schema 1: chưa có layout
pm = Params.sanitize(Params.migrate(old, nil))
check(pm[:layout] == '', 'schema 1 -> layout rỗng, lõi dùng cols')
r = Core.solve(pm)
check(r['ok'] && r['panels'].size > 10 && r['hinges'] == 8, 'tủ cũ dựng đúng (có ngăn kéo/cánh)')
check(Params.migrate({ 'layout' => 'V(0:L[])' }, 2)['layout'] == 'V(0:L[])', 'schema 2 giữ layout')
check(Params.sanitize({ 'back_style' => 'flush' })[:back_style] == 'flush', 'hậu lọt hợp lệ')
check(Params.sanitize({ 'back_style' => 'zzz' })[:back_style] == 'overlay', 'rác -> mặc định')
r2 = Core.solve(Params.sanitize({ 'back_style' => 'flush', 'width' => 1000 }))
check(r2['ok'] && r2['panels'].all? { |p| p['id'] && !p['id'].empty? }, 'mọi tấm có id')
ids = r2['panels'].map { |p| p['id'] }
check(ids.uniq.size == ids.size, 'id duy nhất')
puts 'PARAMS OK'
