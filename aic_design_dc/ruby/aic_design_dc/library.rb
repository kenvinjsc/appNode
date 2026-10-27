# encoding: UTF-8
require 'json'
require 'fileutils'
require_relative 'params'

module AicDesignDC
  # Thư viện mẫu: có sẵn + mẫu người dùng lưu (~/.aic_design_dc/library.json).
  module Library
    BUILTIN = [
      { 'name' => 'Tủ áo 2 khoang + hộc trên', 'kind' => 'cabinet', 'params' => {} },
      { 'name' => 'Tủ áo 4 cánh', 'kind' => 'cabinet', 'params' => {
        'width' => 1800, 'cols' => [[0, 0, 0, 0, 2], [0, 0, 0, 0, 2]] } },
      { 'name' => 'Tủ ngăn kéo 5 tầng', 'kind' => 'cabinet', 'params' => {
        'width' => 800, 'height' => 1100, 'loft_height' => 0, 'loft_doors' => 0, 'cols' => [[0, 5, 0, 0, 0]] } },
      { 'name' => 'Kệ vuông hở', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 800, 'depth' => 300, 'plinth' => 0, 'loft_height' => 0, 'loft_doors' => 0,
        'cols' => [[0, 0, 0, 1, 0], [0, 0, 0, 1, 0], [0, 0, 0, 1, 0]] } },
      { 'name' => 'Tủ bếp dưới', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 810, 'depth' => 560, 'plinth' => 100, 'loft_height' => 0, 'loft_doors' => 0,
        'cols' => [[0, 1, 0, 0, 0], [0, 0, 0, 1, 2]] } },
      { 'name' => 'Tủ giày 3 đợt', 'kind' => 'cabinet', 'params' => {
        'width' => 900, 'height' => 1200, 'depth' => 350, 'loft_height' => 0, 'loft_doors' => 0,
        'cols' => [[0, 0, 0, 3, 2]] } },
      { 'name' => 'Tủ TV thấp (cánh lọt lòng)', 'kind' => 'cabinet', 'params' => {
        'width' => 1800, 'height' => 450, 'depth' => 400, 'door_style' => 'inset', 'loft_height' => 0, 'loft_doors' => 0,
        'cols' => [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0], [0, 0, 0, 0, 1]] } },
      { 'name' => 'Cong 90° R300', 'kind' => 'part', 'params' => { 'kind' => 'bend', 'radius' => 300, 'angle' => 90 } },
      { 'name' => 'Cong 180° R150', 'kind' => 'part', 'params' => { 'kind' => 'bend', 'radius' => 150, 'angle' => 180 } },
      { 'name' => 'Đợt bo 2 góc trước R80', 'kind' => 'part', 'params' => {
        'kind' => 'shelf_round', 'width' => 600, 'depth' => 400, 'corner_fl' => 80, 'corner_fr' => 80 } },
      { 'name' => 'Phào cong lõm 60', 'kind' => 'part', 'params' => { 'kind' => 'moulding', 'profile' => 'cove', 'size_a' => 60, 'size_b' => 60 } },
      { 'name' => 'Phào bậc 80x40', 'kind' => 'part', 'params' => { 'kind' => 'moulding', 'profile' => 'step', 'size_a' => 80, 'size_b' => 40 } },
      { 'name' => 'Xà 60x60', 'kind' => 'part', 'params' => { 'kind' => 'beam', 'size_a' => 60, 'size_b' => 60, 'length' => 1200 } }
    ].freeze

    def self.path
      File.join(Dir.home, '.aic_design_dc', 'library.json')
    end

    def self.user
      JSON.parse(File.read(path, encoding: 'UTF-8'))
    rescue StandardError
      []
    end

    def self.write_user(list)
      FileUtils.mkdir_p(File.dirname(path))
      File.write(path, JSON.pretty_generate(list), encoding: 'UTF-8')
    end

    def self.all
      BUILTIN.map { |e| e.merge('builtin' => true) } + user.map { |e| e.merge('builtin' => false) }
    end

    def self.find(name)
      all.find { |e| e['name'] == name }
    end

    # Lưu/ghi đè mẫu người dùng (không ghi đè mẫu có sẵn).
    def self.save(name, kind, params)
      name = name.to_s.strip
      raise 'Tên mẫu trống' if name.empty?
      raise 'Trùng tên mẫu có sẵn' if BUILTIN.any? { |e| e['name'] == name }
      list = user.reject { |e| e['name'] == name }
      list << { 'name' => name, 'kind' => kind, 'params' => JSON.parse(JSON.generate(params)) }
      write_user(list)
    end

    def self.delete(name)
      list = user
      return false unless list.any? { |e| e['name'] == name }
      write_user(list.reject { |e| e['name'] == name })
      true
    end
  end
end
