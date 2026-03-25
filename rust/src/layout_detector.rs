//! DocLayout-YOLO 版面分析模块
//!
//! 使用 ONNX Runtime 推理 DocLayout-YOLO 模型，检测文档版面元素
//! (标题、正文、图片、表格、公式等)

use crate::geometry::Rect;
use crate::types::LayoutConfig;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;

/// 版面元素类别（DocLayout-YOLO DocStructBench 10 类）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutClass {
    Title = 0,
    PlainText = 1,
    Abandon = 2,
    Figure = 3,
    FigureCaption = 4,
    Table = 5,
    TableCaption = 6,
    TableFootnote = 7,
    IsolateFormula = 8,
    FormulaCaption = 9,
}

impl LayoutClass {
    pub fn from_id(id: usize) -> Option<Self> {
        match id {
            0 => Some(Self::Title),
            1 => Some(Self::PlainText),
            2 => Some(Self::Abandon),
            3 => Some(Self::Figure),
            4 => Some(Self::FigureCaption),
            5 => Some(Self::Table),
            6 => Some(Self::TableCaption),
            7 => Some(Self::TableFootnote),
            8 => Some(Self::IsolateFormula),
            9 => Some(Self::FormulaCaption),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::PlainText => "plain_text",
            Self::Abandon => "abandon",
            Self::Figure => "figure",
            Self::FigureCaption => "figure_caption",
            Self::Table => "table",
            Self::TableCaption => "table_caption",
            Self::TableFootnote => "table_footnote",
            Self::IsolateFormula => "isolate_formula",
            Self::FormulaCaption => "formula_caption",
        }
    }
}

/// 版面检测结果
#[derive(Debug, Clone)]
pub struct LayoutRegion {
    /// 边界框
    pub bbox: Rect,
    /// 类别
    pub class: LayoutClass,
    /// 置信度
    pub confidence: f32,
}

/// DocLayout-YOLO 版面检测器
pub struct LayoutDetector {
    session: Session,
    config: LayoutConfig,
    input_size: u32,
}

impl LayoutDetector {
    /// 创建版面检测器
    pub fn new(config: &LayoutConfig) -> anyhow::Result<Self> {
        let model_path = config
            .model_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Layout model path not set"))?;

        log::info!("Loading DocLayout-YOLO model from: {}", model_path);

        let session = Session::builder()
            .map_err(|e| anyhow::anyhow!("Failed to create session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("Failed to set optimization level: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| anyhow::anyhow!("Failed to set threads: {}", e))?
            .commit_from_file(model_path)
            .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;

        let input_size = config.input_size.unwrap_or(1024);

        log::info!(
            "DocLayout-YOLO model loaded, input_size={}",
            input_size
        );

        Ok(Self {
            session,
            config: config.clone(),
            input_size,
        })
    }

    /// 检测版面元素
    pub fn detect(&mut self, image: &image::DynamicImage) -> anyhow::Result<Vec<LayoutRegion>> {
        let (input_data, padded_h, padded_w, gain, pad_w, pad_h) = self.preprocess(image);

        // Create tensor from raw data with shape [1, 3, H, W]
        let input_tensor = ort::value::Tensor::from_array(
            ([1usize, 3, padded_h, padded_w], input_data),
        )
        .map_err(|e| anyhow::anyhow!("Failed to create input tensor: {}", e))?;

        let outputs = self
            .session
            .run(ort::inputs!["images" => input_tensor])
            .map_err(|e| anyhow::anyhow!("Model inference failed: {}", e))?;

        let first_output = outputs
            .iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No output from layout model"))?;
        let output = first_output
            .1
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow::anyhow!("Failed to extract output tensor: {}", e))?;

        let (shape, data) = output;

        // Output shape: [batch, num_detections, 6] where 6 = [x1, y1, x2, y2, confidence, class_id]
        let num_detections: usize;
        let stride: usize;
        if shape.len() == 3 {
            num_detections = shape[1] as usize;
            stride = shape[2] as usize;
        } else if shape.len() == 2 {
            num_detections = shape[0] as usize;
            stride = shape[1] as usize;
        } else {
            return Err(anyhow::anyhow!("Unexpected output shape: {:?}", shape));
        }

        let confidence_threshold = self.config.confidence_threshold.unwrap_or(0.3);
        let mut regions = Vec::new();

        for i in 0..num_detections {
            let offset = i * stride;
            if offset + 5 >= data.len() {
                break;
            }

            let x1 = data[offset];
            let y1 = data[offset + 1];
            let x2 = data[offset + 2];
            let y2 = data[offset + 3];
            let conf = data[offset + 4];
            let class_id = data[offset + 5] as usize;

            if conf < confidence_threshold {
                continue;
            }

            // Skip zero-padded entries
            if x1 == 0.0 && y1 == 0.0 && x2 == 0.0 && y2 == 0.0 {
                continue;
            }

            let class = match LayoutClass::from_id(class_id) {
                Some(c) => c,
                None => continue,
            };

            // Scale coordinates back to original image
            let orig_x1 = ((x1 - pad_w) / gain).max(0.0_f32);
            let orig_y1 = ((y1 - pad_h) / gain).max(0.0_f32);
            let orig_x2 = ((x2 - pad_w) / gain).min(image.width() as f32);
            let orig_y2 = ((y2 - pad_h) / gain).min(image.height() as f32);

            let w = orig_x2 - orig_x1;
            let h = orig_y2 - orig_y1;

            if w > 0.0 && h > 0.0 {
                regions.push(LayoutRegion {
                    bbox: Rect::new(
                        orig_x1 as f64,
                        orig_y1 as f64,
                        w as f64,
                        h as f64,
                    ),
                    class,
                    confidence: conf,
                });
            }
        }

        log::info!("Layout detection found {} regions", regions.len());
        for r in &regions {
            log::debug!(
                "  {} ({:.2}) at [{:.0}, {:.0}, {:.0}, {:.0}]",
                r.class.name(),
                r.confidence,
                r.bbox.x,
                r.bbox.y,
                r.bbox.width,
                r.bbox.height
            );
        }

        Ok(regions)
    }

    /// 预处理图片：letterbox resize + normalize
    /// 返回 (data, padded_h, padded_w, gain, pad_w, pad_h)
    fn preprocess(&self, image: &image::DynamicImage) -> (Vec<f32>, usize, usize, f32, f32, f32) {
        let (orig_w, orig_h) = (image.width() as f32, image.height() as f32);
        let target = self.input_size as f32;

        // Compute gain (scale factor) — fit the longer side into target
        let gain = (target / orig_w).min(target / orig_h);
        let new_w = (orig_w * gain).round() as u32;
        let new_h = (orig_h * gain).round() as u32;

        // Padding to make dimensions multiples of 32
        let pad_w_half = ((new_w + 31) / 32 * 32 - new_w) as f32 / 2.0;
        let pad_h_half = ((new_h + 31) / 32 * 32 - new_h) as f32 / 2.0;

        let padded_w = new_w as usize + 2 * pad_w_half.ceil() as usize;
        let padded_h = new_h as usize + 2 * pad_h_half.ceil() as usize;

        // Resize
        let resized = image.resize_exact(
            new_w,
            new_h,
            image::imageops::FilterType::Triangle,
        );
        let rgb = resized.to_rgb8();

        // Create CHW buffer filled with 114/255 (YOLO padding)
        let fill_val = 114.0_f32 / 255.0;
        let channel_size = padded_h * padded_w;
        let mut data = vec![fill_val; 3 * channel_size];

        let pad_top = pad_h_half.ceil() as usize;
        let pad_left = pad_w_half.ceil() as usize;

        for y in 0..new_h as usize {
            for x in 0..new_w as usize {
                let pixel = rgb.get_pixel(x as u32, y as u32);
                let py = y + pad_top;
                let px = x + pad_left;
                // CHW layout: channel * H * W + y * W + x
                data[0 * channel_size + py * padded_w + px] = pixel[0] as f32 / 255.0;
                data[1 * channel_size + py * padded_w + px] = pixel[1] as f32 / 255.0;
                data[2 * channel_size + py * padded_w + px] = pixel[2] as f32 / 255.0;
            }
        }

        (data, padded_h, padded_w, gain, pad_w_half.ceil(), pad_h_half.ceil())
    }
}

/// 将版面检测结果映射到 BlockType
pub fn layout_class_to_block_type(class: LayoutClass) -> crate::types::BlockType {
    match class {
        LayoutClass::Title => crate::types::BlockType::QuestionNumber,
        LayoutClass::PlainText => crate::types::BlockType::Text,
        LayoutClass::Figure => crate::types::BlockType::Image,
        LayoutClass::Table => crate::types::BlockType::Table,
        LayoutClass::IsolateFormula => crate::types::BlockType::Text,
        LayoutClass::Abandon => crate::types::BlockType::Unknown,
        LayoutClass::FigureCaption
        | LayoutClass::TableCaption
        | LayoutClass::TableFootnote
        | LayoutClass::FormulaCaption => crate::types::BlockType::Text,
    }
}

/// 使用版面检测结果增强 TextBlock 的分类
pub fn enhance_blocks_with_layout(
    blocks: &mut [crate::types::TextBlock],
    layout_regions: &[LayoutRegion],
) {
    for block in blocks.iter_mut() {
        let block_cx = block.bbox.x + block.bbox.width / 2.0;
        let block_cy = block.bbox.y + block.bbox.height / 2.0;

        // 找到包含该 block 中心点的 layout region（取置信度最高的）
        let mut best_match: Option<&LayoutRegion> = None;
        for region in layout_regions {
            let rx1 = region.bbox.x;
            let ry1 = region.bbox.y;
            let rx2 = rx1 + region.bbox.width;
            let ry2 = ry1 + region.bbox.height;

            if block_cx >= rx1 && block_cx <= rx2 && block_cy >= ry1 && block_cy <= ry2 {
                if best_match.is_none()
                    || region.confidence > best_match.unwrap().confidence
                {
                    best_match = Some(region);
                }
            }
        }

        if let Some(region) = best_match {
            let new_type = layout_class_to_block_type(region.class);
            if block.block_type == crate::types::BlockType::Unknown
                || block.block_type == crate::types::BlockType::Text
            {
                block.block_type = new_type;
            }
        }
    }
}

/// 从版面检测结果中提取图片和表格区域（用于分段时排除或特殊处理）
#[allow(dead_code)]
pub fn extract_non_text_regions(layout_regions: &[LayoutRegion]) -> Vec<LayoutRegion> {
    layout_regions
        .iter()
        .filter(|r| {
            matches!(
                r.class,
                LayoutClass::Figure | LayoutClass::Table | LayoutClass::IsolateFormula
            )
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_class_from_id() {
        assert_eq!(LayoutClass::from_id(0), Some(LayoutClass::Title));
        assert_eq!(LayoutClass::from_id(1), Some(LayoutClass::PlainText));
        assert_eq!(LayoutClass::from_id(5), Some(LayoutClass::Table));
        assert_eq!(LayoutClass::from_id(10), None);
    }

    #[test]
    fn test_layout_class_to_block_type() {
        assert_eq!(
            layout_class_to_block_type(LayoutClass::Title),
            crate::types::BlockType::QuestionNumber
        );
        assert_eq!(
            layout_class_to_block_type(LayoutClass::Figure),
            crate::types::BlockType::Image
        );
        assert_eq!(
            layout_class_to_block_type(LayoutClass::Table),
            crate::types::BlockType::Table
        );
    }

    #[test]
    fn test_enhance_blocks_with_layout() {
        let mut blocks = vec![
            crate::types::TextBlock {
                id: 0,
                bbox: Rect::new(100.0, 100.0, 200.0, 30.0),
                text: None,
                confidence: 0.9,
                block_type: crate::types::BlockType::Unknown,
            },
        ];

        let regions = vec![LayoutRegion {
            bbox: Rect::new(50.0, 50.0, 300.0, 100.0),
            class: LayoutClass::Figure,
            confidence: 0.85,
        }];

        enhance_blocks_with_layout(&mut blocks, &regions);
        assert_eq!(blocks[0].block_type, crate::types::BlockType::Image);
    }
}
