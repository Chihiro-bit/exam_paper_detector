//! OCR 适配层
//!
//! 提供统一的 OCR 接口，支持多种 OCR 引擎

use crate::geometry::Rect;
use crate::types::{OcrConfig, OcrEngine};
use image::DynamicImage;

/// OCR 结果
#[derive(Debug, Clone)]
pub struct OcrResult {
    pub text: String,
    pub bbox: Rect,
    pub confidence: f64,
}

/// OCR 引擎 trait
pub trait OcrEngineT: Send + Sync {
    /// 识别整张图片
    fn recognize(&self, image: &DynamicImage) -> anyhow::Result<Vec<OcrResult>>;

    /// 识别指定区域
    #[allow(dead_code)]
    fn recognize_region(&self, image: &DynamicImage, region: Rect) -> anyhow::Result<String>;
}

/// OCR 适配器
pub struct OcrAdapter {
    engine: Box<dyn OcrEngineT>,
}

impl OcrAdapter {
    /// 创建 OCR 适配器
    pub fn new(config: &OcrConfig) -> anyhow::Result<Self> {
        let engine: Box<dyn OcrEngineT> = match config.engine {
            OcrEngine::Mock => Box::new(MockOcrEngine::new()),
            OcrEngine::Tesseract => {
                // TODO: 实现 Tesseract 适配器
                log::warn!("Tesseract not implemented, using Mock OCR");
                Box::new(MockOcrEngine::new())
            }
        };

        Ok(Self { engine })
    }

    /// 识别图片
    pub fn recognize(&self, image: &DynamicImage) -> anyhow::Result<Vec<OcrResult>> {
        self.engine.recognize(image)
    }

    /// 识别指定区域
    #[allow(dead_code)]
    pub fn recognize_region(&self, image: &DynamicImage, region: Rect) -> anyhow::Result<String> {
        self.engine.recognize_region(image, region)
    }
}

/// Mock OCR 引擎（用于测试）
pub struct MockOcrEngine {
    // 预定义的识别结果
    patterns: Vec<MockPattern>,
}

#[derive(Debug, Clone)]
struct MockPattern {
    bbox: Rect,
    text: String,
}

impl MockOcrEngine {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // 模拟一些题号
                MockPattern {
                    bbox: Rect::new(50.0, 100.0, 30.0, 20.0),
                    text: "1.".to_string(),
                },
                MockPattern {
                    bbox: Rect::new(50.0, 250.0, 30.0, 20.0),
                    text: "2.".to_string(),
                },
                MockPattern {
                    bbox: Rect::new(50.0, 400.0, 30.0, 20.0),
                    text: "3.".to_string(),
                },
                MockPattern {
                    bbox: Rect::new(50.0, 550.0, 30.0, 20.0),
                    text: "4.".to_string(),
                },
                MockPattern {
                    bbox: Rect::new(50.0, 700.0, 30.0, 20.0),
                    text: "5.".to_string(),
                },
            ],
        }
    }
}

impl OcrEngineT for MockOcrEngine {
    fn recognize(&self, _image: &DynamicImage) -> anyhow::Result<Vec<OcrResult>> {
        // 返回预定义的结果
        let results = self
            .patterns
            .iter()
            .map(|p| OcrResult {
                text: p.text.clone(),
                bbox: p.bbox,
                confidence: 0.95,
            })
            .collect();

        Ok(results)
    }

    fn recognize_region(&self, _image: &DynamicImage, region: Rect) -> anyhow::Result<String> {
        // 查找与区域重叠的模式
        for pattern in &self.patterns {
            if pattern.bbox.intersects(&region) {
                return Ok(pattern.text.clone());
            }
        }

        Ok(String::new())
    }
}

// TODO: 实现 Tesseract 适配器
// pub struct TesseractEngine {
//     // tesseract 实例
// }
//
// impl OcrEngineT for TesseractEngine {
//     fn recognize(&self, image: &DynamicImage) -> anyhow::Result<Vec<OcrResult>> {
//         // 调用 tesseract
//         unimplemented!()
//     }
//
//     fn recognize_region(&self, image: &DynamicImage, region: Rect) -> anyhow::Result<String> {
//         unimplemented!()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_ocr() {
        let engine = MockOcrEngine::new();
        let image = DynamicImage::new_rgb8(800, 1000);

        let results = engine.recognize(&image).unwrap();
        assert!(!results.is_empty());

        for result in results {
            println!("OCR: {} at {:?}", result.text, result.bbox);
            assert!(result.text.len() > 0);
        }
    }

    #[test]
    fn test_recognize_region() {
        let engine = MockOcrEngine::new();
        let image = DynamicImage::new_rgb8(800, 1000);

        // 查找第一个题号
        let region = Rect::new(40.0, 90.0, 50.0, 40.0);
        let text = engine.recognize_region(&image, region).unwrap();
        assert_eq!(text, "1.");
    }
}
