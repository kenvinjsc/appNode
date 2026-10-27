# encoding: UTF-8
# Đặt file này + thư mục aic_design_dc/ vào thư mục Plugins của SketchUp
# (kèm hsx_core.dll hoặc libhsx_core.dylib trong aic_design_dc/).
require 'sketchup.rb'
require 'extensions.rb'

module AicDesignDC
  EXTENSION = SketchupExtension.new('AicDesignDC', 'aic_design_dc/main')
  EXTENSION.version = '0.2.0'
  EXTENSION.creator = 'AicDesignDC'
  EXTENSION.description = 'Dựng tủ tham số: vẽ bằng chuột, hộp thoại thông số, kéo giãn. Lõi tính toán C++.'
  Sketchup.register_extension(EXTENSION, true)
end
