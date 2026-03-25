//! 检测器核心模块
//!
//! 协调所有子模块，实现完整的检测流程

use crate::block_detection::{BlockDetector, ColumnInfo};
use crate::layout_detector::{enhance_blocks_with_layout, LayoutDetector};
use crate::ocr::OcrAdapter;
use crate::preprocessing::Preprocessor;
use crate::question_locator::QuestionLocator;
use crate::segmentation::QuestionSegmenter;
use crate::types::*;
use std::time::Instant;

/// 检测器
pub struct Detector {
    config: DetectorConfig,
    preprocessor: Preprocessor,
    block_detector: BlockDetector,
    layout_detector: Option<LayoutDetector>,
    ocr_adapter: Option<OcrAdapter>,
    question_locator: QuestionLocator,
    segmenter: QuestionSegmenter,
}

impl Detector {
    /// 创建检测器
    pub fn new(config: DetectorConfig) -> anyhow::Result<Self> {
        log::info!("Initializing detector...");

        let preprocessor = Preprocessor::new(config.preprocessing.clone());
        let block_detector = BlockDetector::new();

        let layout_detector = if let Some(ref layout_config) = config.layout {
            match LayoutDetector::new(layout_config) {
                Ok(ld) => {
                    log::info!("Layout detector initialized successfully");
                    Some(ld)
                }
                Err(e) => {
                    log::warn!("Failed to initialize layout detector: {}, skipping", e);
                    None
                }
            }
        } else {
            None
        };

        let ocr_adapter = if let Some(ref ocr_config) = config.ocr {
            Some(OcrAdapter::new(ocr_config)?)
        } else {
            None
        };

        let question_locator = QuestionLocator::new(config.question_patterns.clone())?;
        let segmenter = QuestionSegmenter::new();

        Ok(Self {
            config,
            preprocessor,
            block_detector,
            layout_detector,
            ocr_adapter,
            question_locator,
            segmenter,
        })
    }

    /// 处理图片
    pub fn process_image(
        &mut self,
        image_path: &str,
        options: ProcessOptions,
    ) -> anyhow::Result<DetectionResult> {
        log::info!("Processing image: {}", image_path);
        let start_time = Instant::now();

        // Step 1: 图像预处理
        log::info!("Step 1/6: Preprocessing...");
        let preprocessed = self.preprocessor.process(image_path)?;

        if self.config.debug.save_intermediate {
            if let Some(ref output_dir) = self.config.debug.output_dir {
                self.preprocessor
                    .save_debug_images(&preprocessed, output_dir)?;
            }
        }

        // Step 2: Block 检测
        log::info!("Step 2/6: Detecting blocks...");
        let mut blocks = self.block_detector.detect(&preprocessed.binary)?;

        // Step 2.5: 版面分析（可选）
        if let Some(ref mut layout) = self.layout_detector {
            log::info!("Step 2.5/6: Layout analysis (DocLayout-YOLO)...");
            match layout.detect(&preprocessed.original) {
                Ok(regions) => {
                    log::info!("Layout detected {} regions", regions.len());
                    enhance_blocks_with_layout(&mut blocks, &regions);
                }
                Err(e) => {
                    log::warn!("Layout detection failed: {}, continuing without it", e);
                }
            }
        }

        // Step 3: Column detection
        let columns = self.block_detector.detect_columns(&blocks, preprocessed.original.width());
        let num_columns = columns.len();

        let (questions, total_anchors) = if num_columns > 1 {
            log::info!("Detected {} columns, processing per-column", num_columns);
            self.process_multi_column(
                &blocks,
                &columns,
                &preprocessed,
                &options,
            )?
        } else {
            self.process_single_column(
                &blocks,
                &preprocessed,
                &options,
            )?
        };

        let processing_time = start_time.elapsed();

        // 构建结果
        let status = if questions.is_empty() {
            DetectionStatus::Failed
        } else if questions.len() < total_anchors {
            DetectionStatus::PartialSuccess
        } else {
            DetectionStatus::Success
        };

        let metadata = ResultMetadata {
            total_questions: questions.len(),
            processing_time_ms: processing_time.as_millis() as u64,
            image_width: preprocessed.original.width(),
            image_height: preprocessed.original.height(),
            num_columns,
        };

        log::info!(
            "Detection complete: {} questions in {} ms",
            questions.len(),
            metadata.processing_time_ms
        );

        Ok(DetectionResult {
            status,
            questions,
            metadata,
            error: None,
        })
    }

    /// 单栏处理逻辑
    fn process_single_column(
        &self,
        blocks: &[TextBlock],
        preprocessed: &crate::preprocessing::PreprocessedImage,
        options: &ProcessOptions,
    ) -> anyhow::Result<(Vec<QuestionBox>, usize)> {
        // Step 3: OCR 识别（可选）
        log::info!("Step 4/6: OCR recognition...");
        let ocr_results = if let Some(ref ocr) = self.ocr_adapter {
            ocr.recognize(&preprocessed.original)?
        } else {
            vec![]
        };

        // Step 4: 题号定位
        log::info!("Step 5/6: Locating question numbers...");
        let anchors = self.question_locator.locate(
            blocks,
            &ocr_results,
            preprocessed.original.width(),
        )?;

        let total_anchors = anchors.len();

        // Step 5: 题目分段
        log::info!("Step 6/6: Segmenting questions...");
        let questions = if !anchors.is_empty() {
            self.segmenter
                .segment(blocks, &anchors, options.include_debug)?
        } else {
            log::warn!("No question numbers found, using fallback segmentation");
            self.segmenter.fallback_segment(blocks)?
        };

        Ok((questions, total_anchors))
    }

    /// 多栏处理逻辑：分别处理每一栏，然后合并结果
    fn process_multi_column(
        &self,
        blocks: &[TextBlock],
        columns: &[ColumnInfo],
        preprocessed: &crate::preprocessing::PreprocessedImage,
        options: &ProcessOptions,
    ) -> anyhow::Result<(Vec<QuestionBox>, usize)> {
        let mut all_questions: Vec<QuestionBox> = vec![];
        let mut total_anchors = 0;

        // OCR 只需运行一次（整张图）
        log::info!("Step 4/6: OCR recognition (full image)...");
        let ocr_results = if let Some(ref ocr) = self.ocr_adapter {
            ocr.recognize(&preprocessed.original)?
        } else {
            vec![]
        };

        // 按列（从左到右）分别处理
        for column in columns {
            log::info!(
                "Processing column {} (x: {:.0} - {:.0})",
                column.index,
                column.x_start,
                column.x_end
            );

            // 筛选属于当前列的 blocks
            let col_blocks: Vec<TextBlock> = blocks
                .iter()
                .filter(|b| {
                    let block_center_x = b.bbox.x + b.bbox.width / 2.0;
                    block_center_x >= column.x_start && block_center_x < column.x_end
                })
                .cloned()
                .collect();

            if col_blocks.is_empty() {
                log::debug!("Column {} has no blocks, skipping", column.index);
                continue;
            }

            // 筛选属于当前列的 OCR 结果
            let col_ocr: Vec<_> = ocr_results
                .iter()
                .filter(|r| {
                    let center_x = r.bbox.x + r.bbox.width / 2.0;
                    center_x >= column.x_start && center_x < column.x_end
                })
                .cloned()
                .collect();

            // Step 4: 题号定位（per-column）
            let anchors = self.question_locator.locate(
                &col_blocks,
                &col_ocr,
                (column.x_end - column.x_start) as u32,
            )?;
            total_anchors += anchors.len();

            // Step 5: 题目分段（per-column）
            let mut col_questions = if !anchors.is_empty() {
                self.segmenter
                    .segment(&col_blocks, &anchors, options.include_debug)?
            } else {
                log::warn!(
                    "No question numbers in column {}, using fallback",
                    column.index
                );
                self.segmenter.fallback_segment(&col_blocks)?
            };

            // 重新编号：使题号在各栏间连续
            let offset = all_questions.len();
            for q in &mut col_questions {
                // 如果 question_id 是纯数字，重新编号使其连续
                if let Ok(num) = q.question_id.parse::<usize>() {
                    let _ = num; // 保留原始题号（来自 OCR/锚点检测）
                } else if q.recognized_title_text.is_none() {
                    // 仅对 fallback 模式生成的纯索引 ID 重新编号
                    if let Ok(idx) = q.question_id.parse::<usize>() {
                        q.question_id = (idx + offset).to_string();
                    }
                }
            }

            all_questions.extend(col_questions);
        }

        log::info!(
            "Multi-column processing complete: {} questions across {} columns",
            all_questions.len(),
            columns.len()
        );
        Ok((all_questions, total_anchors))
    }

    /// 批量处理图片
    pub fn process_batch(
        &mut self,
        image_paths: Vec<String>,
        options: ProcessOptions,
    ) -> anyhow::Result<Vec<DetectionResult>> {
        let mut results = vec![];

        for (i, path) in image_paths.iter().enumerate() {
            log::info!("Processing image {}/{}: {}", i + 1, image_paths.len(), path);

            match self.process_image(path, options.clone()) {
                Ok(result) => results.push(result),
                Err(e) => {
                    log::error!("Failed to process {}: {}", path, e);
                    results.push(DetectionResult {
                        status: DetectionStatus::Failed,
                        questions: vec![],
                        metadata: ResultMetadata {
                            total_questions: 0,
                            processing_time_ms: 0,
                            image_width: 0,
                            image_height: 0,
                            num_columns: 0,
                        },
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        let config = DetectorConfig::default();
        let detector = Detector::new(config);
        assert!(detector.is_ok());
    }

    // Note: 实际的图像处理测试需要测试图片文件
}
