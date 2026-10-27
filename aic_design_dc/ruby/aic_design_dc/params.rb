# encoding: UTF-8
module AicDesignDC
  # cols: [rộng(0=tự chia), số ngăn kéo, cao ngăn kéo(0=180), số đợt, số cánh]
  DEFAULTS = {
    width: 1600, depth: 560, height: 2400, thick: 17.5, back_thick: 9.5, plinth: 60,
    has_back: true, back_style: 'overlay', door_style: 'overlay', door_thick: 18, gap: 2,
    front_return: 19, loft_height: 400, loft_doors: 1, sheet_w: 1220,
    cols: [[0, 0, 0, 0, 2], [0, 3, 0, 0, 1]]
  }.freeze

  PART_DEFAULTS = {
    kind: 'bend', radius: 300.0, angle: 90.0, part_height: 800.0, part_thick: 17.5,
    width: 600.0, depth: 400.0, corner_fl: 80.0, corner_fr: 80.0, corner_br: 0.0, corner_bl: 0.0,
    length: 1000.0, size_a: 60.0, size_b: 60.0, profile: 'cove'
  }.freeze
  PART_KINDS = %w[bend shelf_round moulding beam].freeze
  PART_PROFILES = %w[cove round bevel step rect].freeze

  LIMITS = { width: [300, 6000], depth: [150, 1200], height: [300, 3500] }.freeze

  module Params
    NUMERIC = %i[width depth height thick back_thick plinth door_thick gap front_return loft_height sheet_w].freeze

    # Nhận Hash (khóa string/symbol, giá trị từ JSON/hộp thoại) -> Hash sạch, đúng kiểu, trong giới hạn.
    def self.sanitize(input, base = DEFAULTS)
      h = base.dup
      (input || {}).each { |k, v| h[k.to_sym] = v }
      NUMERIC.each { |k| h[k] = h[k].to_f }
      %i[loft_doors].each { |k| h[k] = h[k].to_i }
      h[:has_back] = [true, 1, '1', 'true'].include?(h[:has_back])
      h[:back_style] = %w[overlay inset].include?(h[:back_style].to_s) ? h[:back_style].to_s : 'overlay'
      h[:door_style] = %w[overlay half inset].include?(h[:door_style].to_s) ? h[:door_style].to_s : 'overlay'
      LIMITS.each { |k, (lo, hi)| h[k] = [[h[k], lo].max, hi].min }
      h[:cols] = (h[:cols] || []).map do |c|
        a = (c.is_a?(Hash) ? c.values_at('width', 'drawers', 'pitch', 'shelves', 'doors') : c).map(&:to_f)
        [a[0] || 0, (a[1] || 0).to_i, a[2] || 0, (a[3] || 0).to_i, (a[4] || 0).to_i].map { |x| x < 0 ? 0 : x }
      end
      h[:cols] = [[0, 0, 0, 0, 2]] if h[:cols].empty?
      h
    end
  end

  module PartParams
    NUM = %i[radius angle part_height part_thick width depth corner_fl corner_fr corner_br corner_bl length size_a size_b].freeze

    def self.sanitize(input, base = PART_DEFAULTS)
      h = base.dup
      (input || {}).each { |k, v| h[k.to_sym] = v }
      NUM.each { |k| h[k] = [h[k].to_f, 0.0].max }
      h[:kind] = PART_KINDS.include?(h[:kind].to_s) ? h[:kind].to_s : 'bend'
      h[:profile] = PART_PROFILES.include?(h[:profile].to_s) ? h[:profile].to_s : 'cove'
      h[:angle] = [[h[:angle], 1.0].max, 360.0].min
      h
    end
  end

  module State
    @params = DEFAULTS.dup
    @part = PART_DEFAULTS.dup
    class << self
      attr_accessor :params, :part
    end
  end
end
