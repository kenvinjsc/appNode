# encoding: UTF-8
module AicDesignDC
  # cols: [rộng(0=tự chia), số ngăn kéo, cao ngăn kéo(0=180), số đợt, số cánh]
  DEFAULTS = {
    width: 1600, depth: 560, height: 2400, thick: 17.5, back_thick: 9.5, plinth: 60,
    has_back: true, back_style: 'overlay', door_style: 'overlay', door_thick: 18, gap: 2,
    front_return: 19, loft_height: 400, loft_doors: 1,
    cols: [[0, 0, 0, 0, 2], [0, 3, 0, 0, 1]]
  }.freeze

  LIMITS = { width: [300, 6000], depth: [150, 1200], height: [300, 3500] }.freeze

  module Params
    NUMERIC = %i[width depth height thick back_thick plinth door_thick gap front_return loft_height].freeze

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

  module State
    @params = DEFAULTS.dup
    class << self
      attr_accessor :params
    end
  end
end
