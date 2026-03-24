//! 检测器核心模块
//!
//! 协调所有子模块，实现完整的检测流程

use crate::block_detection::BlockDetector;
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
            ocr_adapter,
            question_locator,
            segmenter,
        })
    }

    /// 处理图片
    pub fn process_image(
        &self,
        image_path: &str,
        options: ProcessOptions,
    ) -> anyhow::Result<DetectionResult> {
        log::info!("Processing image: {}", image_path);
        let start_time = Instant::now();

        // Step 1: 图像预处理
        log::info!("Step 1/5: Preprocessing...");
        let preprocessed = self.preprocessor.process(image_path)?;

        if self.config.debug.save_intermediate {
            if let Some(ref output_dir) = self.config.debug.output_dir {
                self.preprocessor
                    .save_debug_images(&preprocessed, output_dir)?;
            }
        }

        // Step 2: Block 检测
        log::info!("Step 2/5: Detecting blocks...");
        let blocks = self.block_detector.detect(&preprocessed.binary)?;

        // Step 3: OCR 识别（可选）
        log::info!("Step 3/5: OCR recognition...");
        let ocr_results = if let Some(ref ocr) = self.ocr_adapter {
            ocr.recognize(&preprocessed.original)?
        } else {
            vec![]
        };

        // Step 4: 题号定位
        log::info!("Step 4/5: Locating question numbers...");
        let anchors = self.question_locator.locate(
            &blocks,
            &ocr_results,
            preprocessed.original.width(),
        )?;

        // Step 5: 题目分段
        log::info!("Step 5/5: Segmenting questions...");
        let questions = if !anchors.is_empty() {
            self.segmenter
                .segment(&blocks, &anchors, options.include_debug)?
        } else {
            // 回退模式
            log::warn!("No question numbers found, using fallback segmentation");
            self.segmenter.fallback_segment(&blocks)?
        };

        let processing_time = start_time.elapsed();

        // 构建结果
        let status = if questions.is_empty() {
            DetectionStatus::Failed
        } else if questions.len() < anchors.len() {
            DetectionStatus::PartialSuccess
        } else {
            DetectionStatus::Success
        };

        let metadata = ResultMetadata {
            total_questions: questions.len(),
            processing_time_ms: processing_time.as_millis() as u64,
            image_width: preprocessed.original.width(),
            image_height: preprocessed.original.height(),
            num_columns: 1, // TODO: 实现分栏检测
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

    /// 批量处理图片
    pub fn process_batch(
        &self,
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
