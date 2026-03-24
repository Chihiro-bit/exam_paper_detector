//! 图像预处理模块
//!
//! 包含：灰度化、去噪、二值化、对比度增强等功能

use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Pixel};
use imageproc::contrast::{threshold as fixed_threshold, ThresholdType};
use std::path::Path;

use crate::types::{BinarizationMethod, PreprocessingConfig};

/// 图像预处理结果
#[derive(Debug)]
pub struct PreprocessedImage {
    /// 原始图像
    pub original: DynamicImage,
    /// 灰度图像
    pub grayscale: GrayImage,
    /// 二值化图像
    pub binary: GrayImage,
    /// 增强后的图像
    pub enhanced: Option<DynamicImage>,
}

/// 图像预处理器
pub struct Preprocessor {
    config: PreprocessingConfig,
}

impl Preprocessor {
    pub fn new(config: PreprocessingConfig) -> Self {
        Self { config }
    }

    /// 加载并预处理图像
    pub fn process(&self, image_path: &str) -> anyhow::Result<PreprocessedImage> {
        log::info!("Loading image from: {}", image_path);

        // 加载图像
        let original = image::open(Path::new(image_path))?;

        log::info!("Image size: {}x{}", original.width(), original.height());

        // 转换为灰度图
        let mut grayscale = self.to_grayscale(&original);

        // 去噪
        if self.config.enable_denoise {
            log::debug!("Applying denoising...");
            grayscale = self.denoise(&grayscale);
        }

        // 对比度增强
        let enhanced = if self.config.contrast_enhancement != 1.0 {
            log::debug!("Applying contrast enhancement...");
            Some(self.enhance_contrast(&original, self.config.contrast_enhancement))
        } else {
            None
        };

        // 二值化
        log::debug!("Applying binarization: {:?}", self.config.binarization_method);
        let binary = self.binarize(&grayscale, self.config.binarization_method);

        Ok(PreprocessedImage {
            original,
            grayscale,
            binary,
            enhanced,
        })
    }

    /// 转换为灰度图
    fn to_grayscale(&self, image: &DynamicImage) -> GrayImage {
        image.to_luma8()
    }

    /// 去噪（高斯模糊）
    fn denoise(&self, image: &GrayImage) -> GrayImage {
        // 使用简单的高斯模糊去噪
        imageproc::filter::gaussian_blur_f32(image, 1.0)
    }

    /// 对比度增强（线性拉伸）
    ///
    /// 以像素均值为中心，按 factor 倍数拉伸对比度。
    /// factor > 1.0 增强对比度，factor < 1.0 降低对比度。
    fn enhance_contrast(&self, image: &DynamicImage, factor: f32) -> DynamicImage {
        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        // 计算灰度均值作为中心
        let mut sum = 0u64;
        let mut count = 0u64;
        for pixel in rgba.pixels() {
            let channels = pixel.channels();
            let gray = (channels[0] as u64 + channels[1] as u64 + channels[2] as u64) / 3;
            sum += gray;
            count += 1;
        }
        let mean = if count > 0 { (sum / count) as f32 } else { 128.0 };

        let mut result = rgba.clone();
        for y in 0..height {
            for x in 0..width {
                let pixel = rgba.get_pixel(x, y);
                let channels = pixel.channels();

                let r = ((channels[0] as f32 - mean) * factor + mean).clamp(0.0, 255.0) as u8;
                let g = ((channels[1] as f32 - mean) * factor + mean).clamp(0.0, 255.0) as u8;
                let b = ((channels[2] as f32 - mean) * factor + mean).clamp(0.0, 255.0) as u8;
                let a = channels[3];

                result.put_pixel(x, y, image::Rgba([r, g, b, a]));
            }
        }

        DynamicImage::ImageRgba8(result)
    }

    /// 二值化
    fn binarize(&self, image: &GrayImage, method: BinarizationMethod) -> GrayImage {
        match method {
            BinarizationMethod::Otsu => self.otsu_binarize(image),
            BinarizationMethod::Adaptive => self.adaptive_binarize(image),
            BinarizationMethod::Fixed => fixed_threshold(image, 128, ThresholdType::Binary),
        }
    }

    /// OTSU 二值化
    fn otsu_binarize(&self, image: &GrayImage) -> GrayImage {
        let otsu_value = self.calculate_otsu_threshold(image);
        log::debug!("OTSU threshold: {}", otsu_value);
        fixed_threshold(image, otsu_value, ThresholdType::Binary)
    }

    /// 计算 OTSU 阈值
    fn calculate_otsu_threshold(&self, image: &GrayImage) -> u8 {
        // 计算直方图
        let mut histogram = vec![0u32; 256];
        for pixel in image.pixels() {
            histogram[pixel[0] as usize] += 1;
        }

        let total_pixels = (image.width() * image.height()) as f64;
        let mut sum_total = 0.0;
        for (i, &count) in histogram.iter().enumerate() {
            sum_total += (i as f64) * (count as f64);
        }

        let mut sum_background = 0.0;
        let mut weight_background = 0.0;
        let mut max_variance = 0.0;
        let mut best_threshold = 0u8;

        for t in 0..256 {
            weight_background += histogram[t] as f64;
            if weight_background == 0.0 {
                continue;
            }

            let weight_foreground = total_pixels - weight_background;
            if weight_foreground == 0.0 {
                break;
            }

            sum_background += (t as f64) * (histogram[t] as f64);

            let mean_background = sum_background / weight_background;
            let mean_foreground = (sum_total - sum_background) / weight_foreground;

            // 计算类间方差
            let variance_between = weight_background * weight_foreground
                * (mean_background - mean_foreground).powi(2);

            if variance_between > max_variance {
                max_variance = variance_between;
                best_threshold = t as u8;
            }
        }

        best_threshold
    }

    /// 自适应二值化
    fn adaptive_binarize(&self, image: &GrayImage) -> GrayImage {
        // 使用局部自适应阈值
        let block_size = 15; // 窗口大小
        let c = 5; // 常数

        let width = image.width();
        let height = image.height();
        let mut result = ImageBuffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                // 计算局部窗口的均值
                let local_mean = self.calculate_local_mean(image, x, y, block_size);

                let pixel_value = image.get_pixel(x, y)[0];
                let threshold_value = if local_mean > c {
                    local_mean - c
                } else {
                    0
                };

                let binary_value = if pixel_value > threshold_value { 255 } else { 0 };
                result.put_pixel(x, y, Luma([binary_value]));
            }
        }

        result
    }

    /// 计算局部均值
    fn calculate_local_mean(&self, image: &GrayImage, cx: u32, cy: u32, size: u32) -> u8 {
        let width = image.width();
        let height = image.height();
        if width == 0 || height == 0 {
            return 128;
        }
        let half_size = size / 2;

        let x_start = cx.saturating_sub(half_size);
        let x_end = (cx + half_size).min(width.saturating_sub(1));
        let y_start = cy.saturating_sub(half_size);
        let y_end = (cy + half_size).min(height.saturating_sub(1));

        let mut sum = 0u32;
        let mut count = 0u32;

        for y in y_start..=y_end {
            for x in x_start..=x_end {
                sum += image.get_pixel(x, y)[0] as u32;
                count += 1;
            }
        }

        if count > 0 {
            (sum / count) as u8
        } else {
            128
        }
    }

    /// 保存中间结果
    pub fn save_debug_images(&self, result: &PreprocessedImage, output_dir: &str) -> anyhow::Result<()> {
        std::fs::create_dir_all(output_dir)?;

        result.grayscale.save(format!("{}/01_grayscale.png", output_dir))?;
        result.binary.save(format!("{}/02_binary.png", output_dir))?;

        if let Some(ref enhanced) = result.enhanced {
            enhanced.save(format!("{}/03_enhanced.png", output_dir))?;
        }

        log::info!("Debug images saved to: {}", output_dir);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otsu_threshold_calculation() {
        // 创建一个简单的测试图像
        let mut img = GrayImage::new(10, 10);

        // 前半部分亮，后半部分暗
        for y in 0..10 {
            for x in 0..10 {
                let value = if x < 5 { 200 } else { 50 };
                img.put_pixel(x, y, Luma([value]));
            }
        }

        let preprocessor = Preprocessor::new(PreprocessingConfig::default());
        let threshold = preprocessor.calculate_otsu_threshold(&img);

        // 阈值应该在 50 和 200 之间（包含端点）
        assert!(threshold >= 50 && threshold < 200);
    }

    #[test]
    fn test_local_mean() {
        let mut img = GrayImage::new(10, 10);

        // 填充固定值
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, Luma([100]));
            }
        }

        let preprocessor = Preprocessor::new(PreprocessingConfig::default());
        let mean = preprocessor.calculate_local_mean(&img, 5, 5, 3);

        assert_eq!(mean, 100);
    }
}
