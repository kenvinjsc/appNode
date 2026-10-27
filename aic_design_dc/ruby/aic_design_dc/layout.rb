# encoding: UTF-8
# Cây bố cục khoang của tủ (khớp cú pháp lõi C++ `core/layout.hpp`) + cú pháp gõ số của công cụ chia.
#   Lá : {'type'=>'leaf', 'd'=>ngăn kéo, 'p'=>cao ngăn, 's'=>đợt, 'l'=>cánh (-1 tự), 'm'=>kiểu cánh 1..3 hoặc nil, 'h'=>bản lề,
#          'f'=>cao tấm fix dưới khoang, 'x'=>rộng ống hút mùi, 'r'=>bản lề cánh đơn 1 trái/2 phải}
#   Chia: {'type'=>'split', 'dir'=>'V'|'H', 'kids'=>[[size_mm, node], ...]}   (size 0 = tự chia đều)
module AicDesignDC
  module Layout
    LEAF_KEYS = %w[d p s l m h f x r].freeze

    def self.leaf(attrs = {})
      { 'type' => 'leaf', 'd' => 0, 'p' => 0, 's' => 0, 'l' => 0, 'm' => nil, 'h' => 0, 'f' => 0, 'x' => 0, 'r' => 0 }.merge(attrs)
    end

    def self.split(dir, kids)
      { 'type' => 'split', 'dir' => dir, 'kids' => kids }
    end

    def self.empty_leaf?(n)
      n['type'] == 'leaf' && n['d'].to_i.zero? && n['s'].to_i.zero? && n['l'].to_i.zero?
    end

    # ---- văn bản <-> cây -------------------------------------------------------------------
    def self.dump(node)
      if node['type'] == 'leaf'
        parts = LEAF_KEYS.map do |k|
          v = node[k]
          next if v.nil? || (v.respond_to?(:zero?) && v.zero?)
          "#{k}=#{fmt(v)}"
        end.compact
        "L[#{parts.join(',')}]"
      else
        "#{node['dir']}(#{node['kids'].map { |sz, kid| "#{fmt(sz)}:#{dump(kid)}" }.join(',')})"
      end
    end

    def self.fmt(v)
      f = v.to_f
      f == f.round ? f.round.to_s : format('%.2f', f).sub(/0+$/, '').sub(/\.$/, '')
    end

    class ParseError < StandardError; end

    def self.parse(text)
      s = text.to_s.strip
      pos = [0]
      node = parse_node(s, pos, 0)
      raise ParseError, 'Thừa ký tự sau nút gốc' unless pos[0] == s.size
      node
    end

    def self.parse_node(s, pos, depth)
      raise ParseError, 'Bố cục lồng quá sâu' if depth > 12
      c = s[pos[0]]
      raise ParseError, 'Bố cục trống' if c.nil?
      pos[0] += 1
      case c
      when 'L' then parse_leaf(s, pos)
      when 'V', 'H'
        raise ParseError, "Cần '(' sau #{c}" unless s[pos[0]] == '('
        pos[0] += 1
        kids = []
        loop do
          m = /\G-?\d+(?:\.\d+)?/.match(s, pos[0]) or raise ParseError, 'Cần một số'
          pos[0] = m.end(0)
          raise ParseError, "Cần ':'" unless s[pos[0]] == ':'
          pos[0] += 1
          kids << [m[0].to_f, parse_node(s, pos, depth + 1)]
          if s[pos[0]] == ',' then pos[0] += 1
          elsif s[pos[0]] == ')' then pos[0] += 1
            break
          else raise ParseError, "Cần ',' hoặc ')'"
          end
        end
        split(c, kids)
      else raise ParseError, "Nút không hợp lệ: #{c}"
      end
    end

    def self.parse_leaf(s, pos)
      raise ParseError, "Cần '['" unless s[pos[0]] == '['
      pos[0] += 1
      n = leaf
      until s[pos[0]] == ']'
        k = s[pos[0]]
        raise ParseError, "Thiếu ']'" if k.nil?
        raise ParseError, "Khóa không hợp lệ: #{k}" unless LEAF_KEYS.include?(k)
        raise ParseError, "Cần '='" unless s[pos[0] + 1] == '='
        m = /\G-?\d+(?:\.\d+)?/.match(s, pos[0] + 2) or raise ParseError, 'Cần một số'
        pos[0] = m.end(0)
        n[k] = %w[p f x].include?(k) ? m[0].to_f : m[0].to_i
        n[k] = nil if k == 'm' && n[k].zero?
        pos[0] += 1 if s[pos[0]] == ','
      end
      pos[0] += 1
      n
    end

    # Tủ kiểu cũ (cols + hộc trên) -> cây tương đương lõi C++ `layoutFromColumns`.
    def self.from_params(prm)
      return parse(prm[:layout]) unless prm[:layout].to_s.strip.empty?
      kids = (prm[:cols] || [[0, 0, 0, 0, 0]]).map do |w, d, pitch, s, l|
        main = leaf('d' => d.to_i, 'p' => pitch.to_f, 's' => s.to_i, 'l' => l.to_i)
        node = if prm[:loft_height].to_f > 0
                 split('H', [[0, main], [prm[:loft_height].to_f, leaf('l' => prm[:loft_doors].to_i)]])
               else main end
        [w.to_f, node]
      end
      split('V', kids)
    end

    # ---- truy cập / sửa --------------------------------------------------------------------
    def self.node_at(root, path)
      path.inject(root) { |n, i| n['kids'][i][1] }
    end

    def self.deep_copy(n)
      Marshal.load(Marshal.dump(n))
    end

    # Trả cây mới với nút tại `path` được thay bằng kết quả của khối.
    def self.update_at(root, path, &blk)
      root = deep_copy(root)
      return blk.call(root) if path.empty?
      parent = node_at(root, path[0...-1])
      parent['kids'][path.last][1] = blk.call(parent['kids'][path.last][1])
      root
    end

    # Chia khoang lá thành các khoang con (nội dung cũ bị bỏ: chia khi khoang còn trống như trong video).
    def self.divide(root, path, dir, sizes)
      update_at(root, path) { |_n| split(dir, sizes.map { |s| [s, leaf] }) }
    end

    def self.set_leaf(root, path, attrs)
      update_at(root, path) do |n|
        raise ArgumentError, 'Khoang này đã được chia nhỏ' unless n['type'] == 'leaf'
        n.merge(attrs)
      end
    end

    # ---- cú pháp gõ số của công cụ chia ----------------------------------------------------
    #   "3"   = 3 vách/đợt chia đều (4 khoang)     "/3" = chia thành 3 khoảng đều (2 vách)
    #   "200" = cách mép gần nhất 200 mm            (số nguyên < 20 hiểu là số lượng)
    # extent: kích thước thông thủy khoang (mm), cursor: vị trí con trỏ tính từ mép thấp (mm), t: dày ván.
    # => mảng kích thước các khoang con (0 = tự chia đều), hoặc nil nếu không hợp lệ.
    def self.parse_divide(input, extent, cursor, t)
      s = input.to_s.strip.tr(',', '.')
      return nil if s.empty?
      if (m = %r{\A/(\d+)\z}.match(s))
        n = m[1].to_i
        return n >= 2 && n <= 20 ? Array.new(n, 0) : nil
      end
      return nil unless s =~ /\A\d+(\.\d+)?\z/
      v = s.to_f
      if s =~ /\A\d+\z/ && v < 20
        n = v.to_i
        return n >= 1 ? Array.new(n + 1, 0) : nil
      end
      return nil if v < 30 || v > extent - t - 30
      cursor < extent / 2.0 ? [v, 0] : [0, v]   # cách mép thấp hay cao tuỳ con trỏ gần mép nào
    end

    # Chia tại vị trí con trỏ (click chuột không gõ số): kích thước khoang thấp = khoảng cách tới mặt gần của ván.
    def self.divide_at(extent, cursor, t)
      lo = (cursor - t / 2.0).round(1)
      lo = [[lo, 30].max, extent - t - 30].min
      [lo, 0]
    end
  end

  # Chọn khoang lá bằng toạ độ mm trong mặt trước của tủ (toạ độ cục bộ).
  module Pick
    # openings: mảng hash từ lõi ({'x0','x1','z0','z1','p'=>path,'t'=>tag...}); trả khoang chứa điểm (hoặc gần nhất trong 30 mm).
    def self.opening_at(openings, x, z)
      hit = openings.find { |o| x >= o['x0'] && x <= o['x1'] && z >= o['z0'] && z <= o['z1'] }
      return hit if hit
      best = openings.min_by { |o| dist(o, x, z) }
      best && dist(best, x, z) <= 30 ? best : nil
    end

    def self.dist(o, x, z)
      dx = [o['x0'] - x, 0, x - o['x1']].max
      dz = [o['z0'] - z, 0, z - o['z1']].max
      Math.hypot(dx, dz)
    end

    # Giao tia (điểm gốc, hướng; toạ độ cục bộ, inch hoặc mm đều được) với mặt phẳng y = 0 -> [x, z] hoặc nil.
    def self.front_plane_hit(origin, dir)
      return nil if dir[1].abs < 1e-9
      k = -origin[1] / dir[1]
      return nil if k < 0
      [origin[0] + k * dir[0], origin[2] + k * dir[2]]
    end
  end
end
