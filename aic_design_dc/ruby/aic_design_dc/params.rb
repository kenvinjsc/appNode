# encoding: UTF-8
module AicDesignDC
  # cols: [rộng(0=tự chia), số ngăn kéo, cao ngăn kéo(0=180), số đợt, số cánh]
  DEFAULTS = {
    width: 1600, depth: 560, height: 2400, thick: 17.5, back_thick: 9.5, plinth: 60,
    has_back: true, back_style: 'overlay', door_style: 'overlay', door_thick: 18, gap: 2,
    cab_type: 'std', drawer_slide: 'side', slide_len: 450, rail_w: 70, rail_count: 2, hang_z: 0, side_on_plinth: false, plinth_front: false, plinth_setback: 50,
    front_return: 19, loft_height: 0, loft_doors: 0, sheet_w: 1220, door_max_w: 600,
    cam: true, band: true, band_thick: 1.0, band_front_thick: 2.0,
    layout: 'V(0:L[])',     # khung rỗng: một khoang trống, chia đợt/vách và thêm ngăn kéo/cánh bằng công cụ
    cols: [[0, 0, 0, 0, 0]]
  }.freeze

  PART_DEFAULTS = {
    kind: 'bend', radius: 300.0, angle: 90.0, part_height: 800.0, part_thick: 17.5,
    width: 600.0, depth: 400.0, height: 1800.0, corner_fl: 80.0, corner_fr: 80.0, corner_br: 0.0, corner_bl: 0.0,
    length: 1000.0, size_a: 60.0, size_b: 60.0, profile: 'cove',
    corner_style: 'round', joint_type: 'none', joint_mode: 'tab', joint_edges: 'F', joint_count: 3,
    joint_len: 40.0, joint_depth: 18.0, joint_margin: 30.0, joint_angle: 10.0, dogbone: 0.0,
    frame_border: 0.0, frame_r: 0.0, frame_style: 'square', rabbet_w: 0.0, rabbet_d: 0.0,
    groove_count: 0, groove_w: 8.0, groove_depth: 8.0, groove_offset: 20.0, groove_spacing: 0.0, groove_inset: 0.0,
    groove_face: 'bottom', kerf_w: 3.0, kerf_remain: 2.0, bend_radius: 300.0, kerf_zone: 0.0, kerf_start: 20.0,
    stile: 60.0, rail: 60.0, frame_thick: 18.0, panel_thick: 6.0, panel_type: 'glass',
    band: true, band_thick: 1.0, band_edges: 'FRBL'
  }.freeze
  PART_KINDS = %w[bend shelf_round moulding beam board kerf_bend door_frame].freeze
  PART_PROFILES = %w[cove round bevel step rect].freeze

  LIMITS = { width: [300, 6000], depth: [150, 1200], height: [300, 3500] }.freeze

  module Params
    NUMERIC = %i[width depth height thick back_thick plinth door_thick gap front_return loft_height sheet_w door_max_w rail_w hang_z slide_len plinth_setback band_thick band_front_thick].freeze

    # Nhận Hash (khóa string/symbol, giá trị từ JSON/hộp thoại) -> Hash sạch, đúng kiểu, trong giới hạn.
    def self.sanitize(input, base = DEFAULTS)
      h = base.dup
      keys = (input || {}).keys.map(&:to_sym)
      (input || {}).each { |k, v| h[k.to_sym] = v }
      h[:layout] = '' if keys.include?(:cols) && !keys.include?(:layout)   # truyền cols (kiểu cũ) mà không truyền layout -> cols có hiệu lực
      NUMERIC.each { |k| h[k] = h[k].to_f }
      %i[loft_doors rail_count].each { |k| h[k] = h[k].to_i }
      %i[has_back cam band side_on_plinth plinth_front].each { |k| h[k] = [true, 1, '1', 'true'].include?(h[k]) }
      h[:cab_type] = %w[std base wall tall].include?(h[:cab_type].to_s) ? h[:cab_type].to_s : 'std'
      h[:drawer_slide] = %w[side under].include?(h[:drawer_slide].to_s) ? h[:drawer_slide].to_s : 'side'
      h[:rail_count] = [[h[:rail_count], 2].max, 6].min
      h[:back_style] = %w[overlay inset].include?(h[:back_style].to_s) ? h[:back_style].to_s : 'overlay'
      h[:door_style] = %w[overlay half inset].include?(h[:door_style].to_s) ? h[:door_style].to_s : 'overlay'
      LIMITS.each { |k, (lo, hi)| h[k] = [[h[k], lo].max, hi].min }
      h[:cols] = (h[:cols] || []).map do |c|
        a = (c.is_a?(Hash) ? c.values_at('width', 'drawers', 'pitch', 'shelves', 'doors') : c).map(&:to_f)
        [a[0] || 0, (a[1] || 0).to_i, a[2] || 0, (a[3] || 0).to_i, (a[4] || 0).to_i].map { |x| x < 0 ? 0 : x }
      end
      h[:cols] = [[0, 0, 0, 0, 0]] if h[:cols].empty?
      h[:layout] = h[:layout].to_s.gsub(/\s+/, '')
      h
    end
  end

  module PartParams
    NUM = %i[radius angle part_height part_thick width depth height corner_fl corner_fr corner_br corner_bl length size_a size_b
             joint_len joint_depth joint_margin joint_angle dogbone frame_border frame_r rabbet_w rabbet_d
             groove_w groove_depth groove_offset groove_spacing groove_inset kerf_w kerf_remain bend_radius kerf_zone kerf_start
             stile rail frame_thick panel_thick band_thick].freeze
    INTS = %i[joint_count groove_count].freeze
    ENUMS = { profile: PART_PROFILES, corner_style: %w[round concave chamfer], joint_type: %w[none finger dovetail tee],
              joint_mode: %w[tab slot], frame_style: %w[square round chamfer], groove_face: %w[bottom top],
              panel_type: %w[glass flat none] }.freeze

    def self.sanitize(input, base = PART_DEFAULTS)
      h = base.dup
      keys = (input || {}).keys.map(&:to_sym)
      (input || {}).each { |k, v| h[k.to_sym] = v }
      h[:layout] = '' if keys.include?(:cols) && !keys.include?(:layout)   # truyền cols (kiểu cũ) mà không truyền layout -> cols có hiệu lực
      NUM.each { |k| h[k] = [h[k].to_f, 0.0].max }
      INTS.each { |k| h[k] = [h[k].to_i, 0].max }
      h[:kind] = PART_KINDS.include?(h[:kind].to_s) ? h[:kind].to_s : 'bend'
      ENUMS.each { |k, list| h[k] = list.include?(h[k].to_s) ? h[k].to_s : PART_DEFAULTS[k] }
      h[:joint_edges] = h[:joint_edges].to_s.upcase.chars.select { |c| 'FRBL'.include?(c) }.uniq.join
      h[:joint_edges] = 'F' if h[:joint_edges].empty?
      h[:band] = [true, 1, '1', 'true'].include?(h[:band])
      h[:band_edges] = h[:band_edges].to_s.upcase.chars.select { |c| 'FRBL'.include?(c) }.uniq.join
      h[:angle] = [[h[:angle], 1.0].max, 360.0].min
      h
    end
  end

  module State
    @params = DEFAULTS.dup
    @part = PART_DEFAULTS.dup
    # Tuỳ chọn các công cụ trên khung tủ (thanh chip như video)
    @tool = { mode: 'shelf', door_style: 1, hinges: 0, door_count: -1, drawer_count: 3, drawer_pitch: 180 }
    class << self
      attr_accessor :params, :part, :tool
    end
  end
end
