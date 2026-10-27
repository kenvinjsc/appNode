# encoding: UTF-8
require 'json'
require 'fileutils'
require_relative 'params'

module AicDesignDC
  # Thư viện mẫu: có sẵn + mẫu người dùng lưu (~/.aic_design_dc/library.json).
  module Library
    # Mẫu KHUNG (rỗng, rồi thêm đợt/vách/ngăn kéo/cánh bằng công cụ) + mẫu tủ có sẵn nội dung để tham khảo.
    BUILTIN = [
      { 'name' => 'Khung tủ áo 1600x2400 (rỗng)', 'kind' => 'cabinet', 'params' => {} },
      { 'name' => 'Khung tủ áo 2 khoang + hộc trên (rỗng)', 'kind' => 'cabinet', 'params' => {
        'layout' => 'V(0:H(0:L[],400:L[]),0:H(0:L[],400:L[]))' } },
      { 'name' => 'Khung tủ áo 4 khoang (rỗng)', 'kind' => 'cabinet', 'params' => {
        'width' => 1800, 'layout' => 'V(0:L[],0:L[],0:L[],0:L[])' } },
      { 'name' => 'Khung tủ bếp dưới (rỗng)', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 810, 'depth' => 560, 'plinth' => 100 } },
      { 'name' => 'Khung kệ hở (rỗng)', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 800, 'depth' => 300, 'plinth' => 0 } },
      { 'name' => 'Khung tủ giày (rỗng)', 'kind' => 'cabinet', 'params' => {
        'width' => 900, 'height' => 1200, 'depth' => 350 } },
      { 'name' => 'Khung tủ TV thấp (rỗng)', 'kind' => 'cabinet', 'params' => {
        'width' => 1800, 'height' => 450, 'depth' => 400, 'door_style' => 'inset' } },
      { 'name' => 'Mẫu đủ: tủ áo 2 khoang + ngăn kéo + hộc trên', 'kind' => 'cabinet', 'params' => {
        'layout' => 'V(0:H(0:L[l=2],400:L[l=1]),0:H(0:L[d=3,l=1],400:L[l=1]))' } },
      { 'name' => 'Mẫu đủ: tủ ngăn kéo 5 tầng', 'kind' => 'cabinet', 'params' => {
        'width' => 800, 'height' => 1100, 'layout' => 'V(0:L[d=5])' } },
      { 'name' => 'Mẫu đủ: tủ bếp dưới', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 810, 'plinth' => 100, 'layout' => 'V(0:L[d=1],0:L[s=1,l=2])' } },
      { 'name' => 'Mẫu đủ: kệ vuông 3 ô', 'kind' => 'cabinet', 'params' => {
        'width' => 1200, 'height' => 800, 'depth' => 300, 'plinth' => 0, 'layout' => 'V(0:L[s=1],0:L[s=1],0:L[s=1])' } },
      { 'name' => 'Cong 90° R300', 'kind' => 'part', 'params' => { 'kind' => 'bend', 'radius' => 300, 'angle' => 90 } },
      { 'name' => 'Cong 180° R150', 'kind' => 'part', 'params' => { 'kind' => 'bend', 'radius' => 150, 'angle' => 180 } },
      { 'name' => 'Đợt bo 2 góc trước R80', 'kind' => 'part', 'params' => {
        'kind' => 'shelf_round', 'width' => 600, 'depth' => 400, 'corner_fl' => 80, 'corner_fr' => 80 } },
      { 'name' => 'Phào cong lõm 60', 'kind' => 'part', 'params' => { 'kind' => 'moulding', 'profile' => 'cove', 'size_a' => 60, 'size_b' => 60 } },
      { 'name' => 'Phào bậc 80x40', 'kind' => 'part', 'params' => { 'kind' => 'moulding', 'profile' => 'step', 'size_a' => 80, 'size_b' => 40 } },
      { 'name' => 'Xà 60x60', 'kind' => 'part', 'params' => { 'kind' => 'beam', 'size_a' => 60, 'size_b' => 60, 'length' => 1200 } },
      { 'name' => 'Tấm bo góc R30 (cung lồi)', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'width' => 600, 'depth' => 400, 'corner_style' => 'round', 'corner_fl' => 30, 'corner_fr' => 30, 'corner_br' => 30, 'corner_bl' => 30 } },
      { 'name' => 'Tấm vát góc 20', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'width' => 600, 'depth' => 400, 'corner_style' => 'chamfer', 'corner_fl' => 20, 'corner_fr' => 20, 'corner_br' => 20, 'corner_bl' => 20 } },
      { 'name' => 'Mộng ngón dương 3 (cạnh trước)', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'joint_type' => 'finger', 'joint_mode' => 'tab', 'joint_edges' => 'F', 'joint_count' => 3 } },
      { 'name' => 'Mộng ngón âm 3 + khử dao Ø6', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'joint_type' => 'finger', 'joint_mode' => 'slot', 'joint_edges' => 'F', 'joint_count' => 3, 'dogbone' => 6 } },
      { 'name' => 'Mộng đuôi én dương', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'joint_type' => 'dovetail', 'joint_mode' => 'tab', 'joint_count' => 3, 'joint_angle' => 10 } },
      { 'name' => 'Mộng chữ T âm + khử dao Ø6', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'joint_type' => 'tee', 'joint_mode' => 'slot', 'joint_count' => 2, 'joint_depth' => 20, 'dogbone' => 6 } },
      { 'name' => 'Đục khung cánh kính (khung 60, hèm 10x8)', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'width' => 500, 'depth' => 700, 'part_thick' => 18, 'frame_border' => 60,
        'frame_style' => 'round', 'frame_r' => 10, 'rabbet_w' => 10, 'rabbet_d' => 8 } },
      { 'name' => 'Ván phay 3 rãnh (rãnh LED 8x9)', 'kind' => 'part', 'params' => {
        'kind' => 'board', 'groove_count' => 3, 'groove_w' => 8, 'groove_depth' => 9 } },
      { 'name' => 'Ván uốn kerf R300', 'kind' => 'part', 'params' => {
        'kind' => 'kerf_bend', 'width' => 600, 'depth' => 400, 'part_thick' => 18, 'bend_radius' => 300 } },
      { 'name' => 'Cánh khung kính 500x1800', 'kind' => 'part', 'params' => {
        'kind' => 'door_frame', 'width' => 500, 'height' => 1800, 'panel_type' => 'glass' } },
      { 'name' => 'Cánh khung tấm phẳng 450x700', 'kind' => 'part', 'params' => {
        'kind' => 'door_frame', 'width' => 450, 'height' => 700, 'panel_type' => 'flat', 'panel_thick' => 9 } }
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
