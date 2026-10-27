# encoding: UTF-8
require 'json'
require_relative 'params'
require_relative 'bridge'
require_relative 'library'

module AicDesignDC
  # Cụm nhiều khối tủ (tủ dưới / treo / cao xếp cạnh nhau, như tủ bếp): dựng từ lõi C++ từng khối, co giãn chung bằng Scale gốc,
  # phào / thanh che (HT) chạy suốt cả hàng.
  #
  # spec = { 'rows' => [ { 'name'=>'Tủ dưới', 'type'=>'base', 'x0'=>0,
  #                        'modules'=>[ { 'w'=>400, 'fixed'=>false, 'params'=>{ 'layout'=>'V(0:L[l=1])' } }, ... ] }, ... ],
  #          'mouldings' => [ { 'name'=>'Phào', 'row'=>'Tủ treo', 'profile'=>'cove', 'size_a'=>50, 'size_b'=>50 } ] }
  module Cluster
    KEY = 'cluster'.freeze

    ROW_DEFAULTS = {
      'base' => { cab_type: 'base', depth: 561.0, height: 866.0, plinth: 75.0, side_on_plinth: true, plinth_front: true, drawer_slide: 'under', hang_z: 0.0 },
      'wall' => { cab_type: 'wall', depth: 331.0, height: 880.0, plinth: 0.0, hang_z: 1495.0 },
      'tall' => { cab_type: 'tall', depth: 561.0, height: 2200.0, plinth: 75.0, side_on_plinth: true, plinth_front: true, drawer_slide: 'under', hang_z: 0.0 }
    }.freeze
    MIN_W = 150.0

    # ---- thuần số (test được không cần SketchUp) -------------------------------------------
    def self.module_params(row, mod)
      base = ROW_DEFAULTS.fetch(row['type'].to_s, ROW_DEFAULTS['base'])
      over = (mod['params'] || {}).each_with_object({}) { |(k, v), h| h[k.to_sym] = v }
      Params.sanitize(base.merge(over).merge(width: mod['w'].to_f), DEFAULTS)
    end

    def self.row_width(row)
      row['modules'].sum { |m| m['w'].to_f }
    end

    def self.total_width(spec)
      spec['rows'].map { |r| r['x0'].to_f + row_width(r) }.max.to_f
    end

    # Chia lại bề rộng tổng: khối `fixed` giữ nguyên, các khối còn lại co giãn tỉ lệ. Không có khối tự do thì co tất cả.
    def self.fit(widths, fixed, total)
      free_idx = widths.each_index.reject { |i| fixed[i] }
      free_idx = widths.each_index.to_a if free_idx.empty?
      fixed_sum = widths.each_index.sum { |i| free_idx.include?(i) ? 0 : widths[i] }
      free_sum = free_idx.sum { |i| widths[i] }
      avail = total - fixed_sum
      raise ArgumentError, "Cụm quá hẹp: còn #{avail.round} mm cho #{free_idx.size} khối" if avail < MIN_W * free_idx.size
      out = widths.dup
      k = avail / free_sum
      free_idx.each { |i| out[i] = (widths[i] * k).round(1) }
      out[free_idx.last] = (out[free_idx.last] + (avail - free_idx.sum { |i| out[i] })).round(1)   # bù sai số làm tròn
      out
    end

    # Kéo giãn cả cụm: xs/ys/zs là hệ số scale theo rộng/sâu/cao. Trả spec mới (không sửa spec cũ).
    def self.resize(spec, xs, ys, zs)
      sp = Marshal.load(Marshal.dump(spec))
      sp['rows'].each do |row|
        old = row['modules'].map { |m| m['w'].to_f }
        neww = (xs - 1).abs < 1e-6 ? old : fit(old, row['modules'].map { |m| m['fixed'] }, row_width(row) * xs)
        row['x0'] = (row['x0'].to_f * xs).round(1)
        row['modules'].each_with_index do |m, i|
          cur = module_params(row, m)
          pr = (m['params'] ||= {})
          pr['depth'] = (cur[:depth] * ys).round(1) if (ys - 1).abs > 1e-6
          if (zs - 1).abs > 1e-6
            pr['height'] = (cur[:height] * zs).round(1)
            pr['hang_z'] = (cur[:hang_z] * zs).round(1) if cur[:hang_z] > 0
          end
          m['w'] = neww[i]
        end
      end
      sp
    end

    # Vị trí từng khối: [{row, mod, x, params}] (x tính từ gốc cụm, mm)
    def self.placements(spec)
      spec['rows'].flat_map do |row|
        x = row['x0'].to_f
        row['modules'].map do |m|
          pl = { row: row, mod: m, x: x, params: module_params(row, m) }
          x += m['w'].to_f
          pl
        end
      end
    end

    # Phào / thanh che chạy suốt hàng: [{m, x0, len, z, y}] (mm). `ext_l`/`ext_r`: nhô ra hai đầu hàng.
    def self.mouldings(spec)
      (spec['mouldings'] || []).filter_map do |ml|
        row = spec['rows'].find { |r| r['name'] == ml['row'] }
        next unless row && !row['modules'].empty?
        prm = row['modules'].map { |m| module_params(row, m) }
        top = prm.map { |p| p[:hang_z] + p[:height] }.max
        el = ml['ext_l'].to_f
        { m: ml, x0: row['x0'].to_f - el, len: row_width(row) + el + ml['ext_r'].to_f, z: top, y: -prm.first[:door_thick] }
      end
    end

    # ---- dựng trong SketchUp -------------------------------------------------------------------
    def self.cluster?(e)
      e.is_a?(Sketchup::Group) && !e.get_attribute('hsx', KEY).nil?
    end

    def self.spec_of(grp)
      JSON.parse(grp.get_attribute('hsx', KEY))
    end

    def self.scaled?(grp)
      tr = grp.transformation
      [tr.xscale, tr.yscale, tr.zscale].any? { |v| (v - 1).abs > 1e-4 }
    end

    def self.create(model, spec, transformation = nil, operation: true)
      model.start_operation('AicDesignDC: Cụm tủ', true) if operation
      grp = model.active_entities.add_group
      grp.name = 'Cụm tủ'
      placements(spec).each do |pl|
        Builder.build(model, pl[:params], Geom::Transformation.translation([pl[:x].mm, 0, 0]), operation: false, parent: grp.entities)
      end
      mouldings(spec).each do |mo|
        res = Core.solve(PartParams.sanitize('kind' => 'moulding', 'profile' => mo[:m]['profile'] || 'cove', 'length' => mo[:len],
                                              'size_a' => mo[:m]['size_a'] || 50, 'size_b' => mo[:m]['size_b'] || 50))
        raise res['error'].to_s unless res['ok']
        mg = grp.entities.add_group
        mg.name = mo[:m]['name'] || 'Phào'
        res['panels'].each { |pn| Builder.add_panel(mg.entities, pn, model) }
        mg.transformation = Geom::Transformation.translation([mo[:x0].mm, mo[:y].mm, mo[:z].mm])
      end
      grp.transformation = transformation if transformation
      grp.set_attribute('hsx', KEY, JSON.generate(spec))
      model.commit_operation if operation
      grp
    rescue StandardError
      model.abort_operation if operation
      raise
    end

    def self.rebuild(model, grp, spec, transformation = nil, transparent: false)
      tr = transformation || grp.transformation
      model.start_operation('AicDesignDC: Sửa cụm tủ', true, false, transparent)
      begin
        grp.erase!
        new_grp = create(model, spec, tr, operation: false)
        model.commit_operation
        new_grp
      rescue StandardError
        model.abort_operation
        raise
      end
    end

    # Scale gốc trên cả cụm -> chia lại bề rộng các khối, dựng lại từng khối từ lõi (ván giữ độ dày).
    def self.normalize_scale(model, grp)
      tr = grp.transformation
      spec = resize(spec_of(grp), tr.xscale, tr.yscale, tr.zscale)
      rigid = Geom::Transformation.axes(tr.origin, tr.xaxis.normalize, tr.yaxis.normalize, tr.zaxis.normalize)
      rebuild(model, grp, spec, rigid, transparent: true)
    end

    def self.sample_kitchen
      Library.kitchen_spec
    end

    # Bảng cắt gộp của cả cụm (các khối + phào).
    def self.cutlist(spec)
      rows = {}
      placements(spec).each do |pl|
        Core.solve(pl[:params])['cutlist'].each do |r|
          key = r.values_at('role', 'L', 'W', 'T', 'band')
          rows[key] ? rows[key]['qty'] += r['qty'] : rows[key] = r.dup
        end
      end
      rows.values
    end
  end
end
