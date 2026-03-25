//! 图像预处理模块
//!
//! 包含：灰度化、去噪、二值化、对比度增强等功能

use image::{DynamicImage, GrayImage, ImageBuffer, Luma, Pixel, Rgba};
use imageproc::contrast::{threshold as fixed_threshold, ThresholdType};
use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
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
        let mut original = image::open(Path::new(image_path))?;

        log::info!("Image size: {}x{}", original.width(), original.height());

        // 转换为灰度图
        let mut grayscale = self.to_grayscale(&original);

        // 倾斜校正
        if self.config.enable_deskew {
            log::debug!("Applying deskew...");
            let (deskewed_gray, angle) = self.deskew(&grayscale);
            if angle.abs() > 0.01 {
                log::info!("Detected skew angle: {:.2}°, correcting...", angle);
                grayscale = deskewed_gray;
                // Also rotate the original image by the same angle
                let rgba = original.to_rgba8();
                let rotated_rgba = rotate_about_center(
                    &rgba,
                    angle.to_radians() as f32,
                    Interpolation::Bilinear,
                    Rgba([255u8, 255u8, 255u8, 255u8]),
                );
                original = DynamicImage::ImageRgba8(rotated_rgba);
            } else {
                log::debug!("No significant skew detected (angle: {:.4}°)", angle);
            }
        }

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

    /// 自适应二值化（使用积分图实现 O(1) 局部均值查询）
    fn adaptive_binarize(&self, image: &GrayImage) -> GrayImage {
        let width = image.width();
        let height = image.height();

        // 动态窗口大小：根据图像分辨率自适应
        let min_dim = width.min(height);
        let block_size = 15u32.max(min_dim / 40);
        let half_size = block_size / 2;
        let c: u8 = 10; // 常数（噪声容忍度）

        log::debug!(
            "Adaptive binarize: image {}x{}, block_size={}, c={}",
            width, height, block_size, c
        );

        // 计算积分图
        let integral = self.compute_integral_image(image);

        let mut result = ImageBuffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let local_mean =
                    Self::query_integral_mean(&integral, x, y, half_size, width, height);

                let pixel_value = image.get_pixel(x, y)[0];
                let threshold_value = local_mean.saturating_sub(c);

                let binary_value = if pixel_value > threshold_value { 255 } else { 0 };
                result.put_pixel(x, y, Luma([binary_value]));
            }
        }

        result
    }

    /// 计算积分图（Summed Area Table）
    ///
    /// integral[y][x] = sum of all pixels in image[0..=y-1][0..=x-1]
    /// 使用 1-indexed 以简化边界处理（第 0 行/列为 0）
    fn compute_integral_image(&self, image: &GrayImage) -> Vec<Vec<u64>> {
        let width = image.width() as usize;
        let height = image.height() as usize;

        // (height+1) x (width+1)，第 0 行和第 0 列全为 0
        let mut integral = vec![vec![0u64; width + 1]; height + 1];

        for y in 0..height {
            let mut row_sum = 0u64;
            for x in 0..width {
                row_sum += image.get_pixel(x as u32, y as u32)[0] as u64;
                integral[y + 1][x + 1] = integral[y][x + 1] + row_sum;
            }
        }

        integral
    }

    /// 使用积分图进行 O(1) 局部均值查询
    ///
    /// 查询以 (x, y) 为中心、半径为 half_size 的窗口内像素均值
    fn query_integral_mean(
        integral: &[Vec<u64>],
        x: u32,
        y: u32,
        half_size: u32,
        width: u32,
        height: u32,
    ) -> u8 {
        // 计算窗口边界（clamp 到图像范围）
        let x1 = x.saturating_sub(half_size) as usize;
        let y1 = y.saturating_sub(half_size) as usize;
        let x2 = ((x + half_size) as usize).min((width as usize).saturating_sub(1));
        let y2 = ((y + half_size) as usize).min((height as usize).saturating_sub(1));

        if x2 < x1 || y2 < y1 {
            return 128;
        }

        let count = ((x2 - x1 + 1) * (y2 - y1 + 1)) as u64;
        if count == 0 {
            return 128;
        }

        // 积分图查询：integral 是 1-indexed，所以 +1 偏移
        // 使用 wrapping 算术避免中间步骤溢出（inclusion-exclusion 保证最终结果正确）
        let sum = integral[y2 + 1][x2 + 1]
            .wrapping_sub(integral[y1][x2 + 1])
            .wrapping_sub(integral[y2 + 1][x1])
            .wrapping_add(integral[y1][x1]);

        (sum / count) as u8
    }

    /// 倾斜校正
    ///
    /// 使用水平投影方差法检测倾斜角度，并旋转校正。
    /// 返回校正后的灰度图和检测到的角度（度）。
    fn deskew(&self, image: &GrayImage) -> (GrayImage, f64) {
        let (width, height) = image.dimensions();

        // 对于大图像，缩小以加速角度检测
        let scale = if height > 800 {
            800.0 / height as f64
        } else {
            1.0
        };

        let small = if scale < 1.0 {
            let new_w = (width as f64 * scale) as u32;
            let new_h = (height as f64 * scale) as u32;
            image::imageops::resize(image, new_w, new_h, image::imageops::FilterType::Nearest)
        } else {
            image.clone()
        };

        // 候选角度：-5° 到 5°，步长 0.5°
        let mut best_angle = 0.0f64;
        let mut best_variance = 0.0f64;

        let mut angle = -5.0f64;
        while angle <= 5.0 {
            let radians = angle.to_radians() as f32;
            let rotated = rotate_about_center(
                &small,
                radians,
                Interpolation::Bilinear,
                Luma([255u8]),
            );

            let variance = Self::horizontal_projection_variance(&rotated);

            if variance > best_variance {
                best_variance = variance;
                best_angle = angle;
            }

            angle += 0.5;
        }

        // 精细搜索：在最佳角度附近 +/- 0.5° 内以 0.1° 步长搜索
        let fine_start = best_angle - 0.5;
        let fine_end = best_angle + 0.5;
        let mut angle = fine_start;
        while angle <= fine_end {
            let radians = angle.to_radians() as f32;
            let rotated = rotate_about_center(
                &small,
                radians,
                Interpolation::Bilinear,
                Luma([255u8]),
            );

            let variance = Self::horizontal_projection_variance(&rotated);

            if variance > best_variance {
                best_variance = variance;
                best_angle = angle;
            }

            angle += 0.1;
        }

        log::debug!("Deskew: best angle = {:.2}°, variance = {:.2}", best_angle, best_variance);

        // 对原始大小图像应用旋转
        if best_angle.abs() < 0.01 {
            return (image.clone(), 0.0);
        }

        let corrected = rotate_about_center(
            image,
            best_angle.to_radians() as f32,
            Interpolation::Bilinear,
            Luma([255u8]),
        );

        (corrected, best_angle)
    }

    /// 计算水平投影的方差
    ///
    /// 水平投影 = 每行的黑色像素计数。文本行对齐时方差最大。
    fn horizontal_projection_variance(image: &GrayImage) -> f64 {
        let (width, height) = image.dimensions();
        let mut projections = Vec::with_capacity(height as usize);

        for y in 0..height {
            let mut dark_count = 0u32;
            for x in 0..width {
                if image.get_pixel(x, y)[0] < 128 {
                    dark_count += 1;
                }
            }
            projections.push(dark_count as f64);
        }

        if projections.is_empty() {
            return 0.0;
        }

        let n = projections.len() as f64;
        let mean = projections.iter().sum::<f64>() / n;
        let variance = projections.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n;

        variance
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
    fn test_integral_image_mean() {
        let mut img = GrayImage::new(10, 10);

        // 填充固定值
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, Luma([100]));
            }
        }

        let preprocessor = Preprocessor::new(PreprocessingConfig::default());
        let integral = preprocessor.compute_integral_image(&img);
        let mean = Preprocessor::query_integral_mean(&integral, 5, 5, 1, 10, 10);

        assert_eq!(mean, 100);
    }

    #[test]
    fn test_integral_image_correctness() {
        // 创建一个非均匀图像验证积分图正确性
        let mut img = GrayImage::new(4, 4);
        // Row 0: [10, 20, 30, 40]
        // Row 1: [50, 60, 70, 80]
        // Row 2: [90,100,110,120]
        // Row 3: [130,140,150,160]
        for y in 0..4u32 {
            for x in 0..4u32 {
                let val = ((y * 4 + x) * 10 + 10) as u8;
                img.put_pixel(x, y, Luma([val]));
            }
        }

        let preprocessor = Preprocessor::new(PreprocessingConfig::default());
        let integral = preprocessor.compute_integral_image(&img);

        // 全图均值：(10+20+...+160)/16 = 1360/16 = 85
        let mean = Preprocessor::query_integral_mean(&integral, 1, 1, 10, 4, 4);
        assert_eq!(mean, 85);

        // 单像素查询 (half_size=0): pixel at (0,0) = 10
        let single = Preprocessor::query_integral_mean(&integral, 0, 0, 0, 4, 4);
        assert_eq!(single, 10);
    }

    #[test]
    fn test_horizontal_projection_variance() {
        // 均匀图像应该有0方差
        let img = GrayImage::from_fn(10, 10, |_x, _y| Luma([128]));
        let variance = Preprocessor::horizontal_projection_variance(&img);
        assert!(variance < 0.001, "Uniform image should have ~0 variance");

        // 交替行黑白应该有高方差
        let img = GrayImage::from_fn(10, 10, |_x, y| {
            if y % 2 == 0 { Luma([0]) } else { Luma([255]) }
        });
        let variance = Preprocessor::horizontal_projection_variance(&img);
        assert!(variance > 0.0, "Alternating rows should have positive variance");
    }
}
