//! Flutter Rust Bridge API
//!
//! 这些函数会被 flutter_rust_bridge 自动生成对应的 Dart 绑定

use crate::detector::Detector;
use crate::types::*;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

/// 全局检测器实例（线程安全）
static DETECTOR: OnceLock<Mutex<Option<Detector>>> = OnceLock::new();

/// 获取或初始化全局 Mutex
fn get_detector_mutex() -> &'static Mutex<Option<Detector>> {
    DETECTOR.get_or_init(|| Mutex::new(None))
}

/// 初始化检测器
///
/// # Arguments
/// * `config_json` - JSON 格式的配置，空字符串则使用默认配置
///
/// # Returns
/// 成功返回 true
pub fn init_detector(config_json: String) -> Result<bool, String> {
    // 初始化日志（只会初始化一次，重复调用无副作用）
    let _ = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .try_init();

    log::info!("Initializing detector with config length: {}", config_json.len());

    // 解析配置
    let config: DetectorConfig = if config_json.is_empty() {
        DetectorConfig::default()
    } else {
        serde_json::from_str(config_json.as_str())
            .map_err(|e| format!("Failed to parse config: {}", e))?
    };

    // 创建检测器
    let detector =
        Detector::new(config).map_err(|e| format!("Failed to create detector: {}", e))?;

    // 保存到全局实例（线程安全）
    let mut guard = get_detector_mutex()
        .lock()
        .map_err(|e| format!("Failed to lock detector mutex: {}", e))?;
    *guard = Some(detector);

    log::info!("Detector initialized successfully");
    Ok(true)
}

/// 处理单张图片（返回 JSON 字符串）
///
/// # Arguments
/// * `image_path` - 图片文件路径
/// * `options_json` - 处理选项（JSON 格式，空字符串使用默认选项）
///
/// # Returns
/// JSON 格式的完整检测结果
pub fn process_image(image_path: String, options_json: String) -> Result<String, String> {
    log::info!("Processing image: {}", image_path);

    let guard = get_detector_mutex()
        .lock()
        .map_err(|e| format!("Failed to lock detector: {}", e))?;

    let detector = guard
        .as_ref()
        .ok_or_else(|| "Detector not initialized. Call init_detector first.".to_string())?;

    // 解析选项
    let options: ProcessOptions = if options_json.is_empty() {
        ProcessOptions::default()
    } else {
        serde_json::from_str(options_json.as_str())
            .map_err(|e| format!("Failed to parse options: {}", e))?
    };

    let result = detector
        .process_image(&image_path, options)
        .map_err(|e| format!("Detection failed: {}", e))?;

    let result_json = serde_json::to_string(&result)
        .map_err(|e| format!("Failed to serialize result: {}", e))?;

    log::info!(
        "Detection complete, found {} questions",
        result.questions.len()
    );
    Ok(result_json)
}

/// 批量处理多张图片
///
/// # Arguments
/// * `image_paths_json` - 图片路径列表（JSON 数组格式）
/// * `options_json` - 处理选项
///
/// # Returns
/// JSON 格式的检测结果数组
pub fn process_batch(image_paths_json: String, options_json: String) -> Result<String, String> {
    log::info!("Processing batch");

    let guard = get_detector_mutex()
        .lock()
        .map_err(|e| format!("Failed to lock detector: {}", e))?;

    let detector = guard
        .as_ref()
        .ok_or_else(|| "Detector not initialized. Call init_detector first.".to_string())?;

    let image_paths: Vec<String> = serde_json::from_str(image_paths_json.as_str())
        .map_err(|e| format!("Failed to parse image paths: {}", e))?;

    let options: ProcessOptions = if options_json.is_empty() {
        ProcessOptions::default()
    } else {
        serde_json::from_str(options_json.as_str())
            .map_err(|e| format!("Failed to parse options: {}", e))?
    };

    let results = detector
        .process_batch(image_paths, options)
        .map_err(|e| format!("Batch processing failed: {}", e))?;

    let results_json = serde_json::to_string(&results)
        .map_err(|e| format!("Failed to serialize results: {}", e))?;

    Ok(results_json)
}

/// 获取默认配置（JSON 格式）
pub fn get_default_config() -> Result<String, String> {
    let config = DetectorConfig::default();
    serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))
}

/// 获取版本信息
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// 释放检测器资源
pub fn dispose_detector() -> Result<bool, String> {
    log::info!("Disposing detector");

    let mut guard = get_detector_mutex()
        .lock()
        .map_err(|e| format!("Failed to lock detector: {}", e))?;
    *guard = None;

    log::info!("Detector disposed");
    Ok(true)
}

// ============== flutter_rust_bridge 会直接映射以下类型到 Dart ==============

/// 简化的检测结果（直接通过 FRB 传递给 Dart，无需 JSON 中间层）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleDetectionResult {
    pub success: bool,
    pub question_count: i32,
    pub processing_time_ms: i64,
    pub questions: Vec<SimpleQuestionBox>,
    pub error_message: Option<String>,
}

/// 简化的题目框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleQuestionBox {
    pub question_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub confidence: f64,
}

/// 处理单张图片并返回结构化结果（推荐使用此 API）
///
/// 相比 `process_image`，此函数直接返回结构体，
/// flutter_rust_bridge 会自动映射为 Dart 类，无需手动 JSON 解析。
pub fn process_image_simple(
    image_path: String,
    include_debug: bool,
) -> Result<SimpleDetectionResult, String> {
    // Use into_inner() to recover from a poisoned mutex
    let guard = match get_detector_mutex().lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    let detector = match guard.as_ref() {
        Some(d) => d,
        None => {
            return Ok(SimpleDetectionResult {
                success: false,
                question_count: 0,
                processing_time_ms: 0,
                questions: vec![],
                error_message: Some(
                    "Detector not initialized. Call init_detector first.".to_string(),
                ),
            });
        }
    };

    let options = ProcessOptions {
        page_index: 0,
        include_debug,
        save_intermediate: false,
    };

    match detector.process_image(&image_path, options) {
        Ok(result) => {
            let simple_questions: Vec<SimpleQuestionBox> = result
                .questions
                .iter()
                .map(|q| SimpleQuestionBox {
                    question_id: q.question_id.clone(),
                    x: q.bounding_box.x,
                    y: q.bounding_box.y,
                    width: q.bounding_box.width,
                    height: q.bounding_box.height,
                    confidence: q.confidence,
                })
                .collect();

            Ok(SimpleDetectionResult {
                success: matches!(
                    result.status,
                    DetectionStatus::Success | DetectionStatus::PartialSuccess
                ),
                question_count: result.questions.len() as i32,
                processing_time_ms: result.metadata.processing_time_ms as i64,
                questions: simple_questions,
                error_message: result.error,
            })
        }
        Err(e) => Ok(SimpleDetectionResult {
            success: false,
            question_count: 0,
            processing_time_ms: 0,
            questions: vec![],
            error_message: Some(e.to_string()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_config() {
        let config_json = get_default_config().unwrap();
        assert!(!config_json.is_empty());

        // 确保能反序列化回来
        let config: DetectorConfig = serde_json::from_str(&config_json).unwrap();
        assert!(config.question_patterns.len() > 0);
    }

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
        assert_eq!(version, "0.1.0");
    }

    #[test]
    fn test_init_and_dispose() {
        let result = init_detector(String::new());
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result = dispose_detector();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_process_without_init() {
        // 确保先 dispose
        let _ = dispose_detector();

        let result = process_image_simple("test.jpg".to_string(), false);
        assert!(result.is_ok());
        // 返回的 SimpleDetectionResult 应该包含错误
        let r = result.unwrap();
        assert!(!r.success);
        assert!(r.error_message.is_some());
    }

    #[test]
    fn test_diagnose_real_image() {
        use crate::block_detection::BlockDetector;
        use crate::preprocessing::Preprocessor;
        use crate::question_locator::QuestionLocator;

        let image_path = r"C:\Users\Administrator\Downloads\151722_57ce6d8297914.jpg";
        if !std::path::Path::new(image_path).exists() {
            println!("Test image not found, skipping diagnostic test");
            return;
        }

        // Step 1: 预处理
        let config = DetectorConfig::default();
        let preprocessor = Preprocessor::new(config.preprocessing.clone());
        let preprocessed = preprocessor.process(image_path).unwrap();
        println!("=== Image: {}x{} ===", preprocessed.original.width(), preprocessed.original.height());

        // Step 2: Block 检测
        let detector = BlockDetector::new();
        let blocks = detector.detect(&preprocessed.binary).unwrap();
        println!("\n=== Detected {} blocks ===", blocks.len());
        for b in &blocks {
            println!("  Block {}: ({:.0}, {:.0}) {}x{:.0}",
                b.id, b.bbox.x, b.bbox.y, b.bbox.width, b.bbox.height);
        }

        // Step 3: 题号定位
        let locator = QuestionLocator::new(config.question_patterns.clone()).unwrap();

        // 查看几何推断的候选 blocks
        let left_margin = preprocessed.original.width() as f64 * 0.15;
        println!("\n=== Candidate blocks (x < {:.0}, w < 100, h < 50) ===", left_margin);
        let mut candidates: Vec<_> = blocks.iter()
            .filter(|b| b.bbox.x < left_margin && b.bbox.width < 100.0 && b.bbox.height < 50.0)
            .collect();
        candidates.sort_by(|a, b| a.bbox.y.partial_cmp(&b.bbox.y).unwrap());
        for c in &candidates {
            println!("  Block {}: ({:.0}, {:.0}) {:.0}x{:.0}",
                c.id, c.bbox.x, c.bbox.y, c.bbox.width, c.bbox.height);
        }

        // 也看看宽松条件下的左侧 blocks
        println!("\n=== All blocks with x < {:.0} ===", left_margin);
        let mut left_blocks: Vec<_> = blocks.iter()
            .filter(|b| b.bbox.x < left_margin)
            .collect();
        left_blocks.sort_by(|a, b| a.bbox.y.partial_cmp(&b.bbox.y).unwrap());
        for b in &left_blocks {
            println!("  Block {}: ({:.0}, {:.0}) {:.0}x{:.0}",
                b.id, b.bbox.x, b.bbox.y, b.bbox.width, b.bbox.height);
        }

        // 实际定位结果
        let anchors = locator.locate(&blocks, &[], preprocessed.original.width()).unwrap();
        println!("\n=== Located {} anchors ===", anchors.len());
        for a in &anchors {
            println!("  Q{}: ({:.0}, {:.0}) {:.0}x{:.0} conf={:.2} text='{}'",
                a.question_id, a.bbox.x, a.bbox.y, a.bbox.width, a.bbox.height,
                a.confidence, a.text);
        }

        // 完整检测
        let _ = init_detector(String::new());
        let result = process_image_simple(image_path.to_string(), true).unwrap();
        println!("\n=== Final Result ===");
        println!("Success: {}, Questions: {}", result.success, result.question_count);
        for q in &result.questions {
            println!("  Q{}: ({:.0}, {:.0}) {:.0}x{:.0} conf={:.2}",
                q.question_id, q.x, q.y, q.width, q.height, q.confidence);
        }
        let _ = dispose_detector();
    }

    #[test]
    fn test_end_to_end_with_generated_image() {
        use image::{GrayImage, Luma};

        // 创建一张模拟试卷图片 (800x1200, 白底黑字)
        let width = 800u32;
        let height = 1200u32;
        let mut img = GrayImage::from_pixel(width, height, Luma([255u8]));

        // 画几个"题号"区域 (黑色矩形模拟文字块)
        // 题目1区域: y=50..100
        for x in 30..60 {
            for y in 50..80 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }
        // 题目1内容: y=50..250
        for x in 80..700 {
            for y in 55..70 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }
        for x in 80..600 {
            for y in 90..105 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }

        // 题目2区域: y=300..350
        for x in 30..60 {
            for y in 300..330 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }
        for x in 80..650 {
            for y in 305..320 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }
        for x in 80..550 {
            for y in 340..355 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }

        // 题目3区域: y=550..600
        for x in 30..60 {
            for y in 550..580 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }
        for x in 80..700 {
            for y in 555..570 {
                img.put_pixel(x, y, Luma([0u8]));
            }
        }

        // 保存到临时文件
        let temp_path = std::env::temp_dir().join("test_exam_paper.png");
        img.save(&temp_path).expect("Failed to save test image");

        // 初始化检测器
        let _ = init_detector(String::new());

        // 处理图片
        let result = process_image_simple(
            temp_path.to_string_lossy().to_string(),
            true,
        );

        assert!(result.is_ok(), "process_image_simple failed: {:?}", result.err());
        let r = result.unwrap();

        println!("=== End-to-End Test Result ===");
        println!("Success: {}", r.success);
        println!("Question count: {}", r.question_count);
        println!("Processing time: {}ms", r.processing_time_ms);
        if let Some(ref err) = r.error_message {
            println!("Error: {}", err);
        }
        for q in &r.questions {
            println!(
                "  Question {}: ({:.0}, {:.0}) {}x{:.0} confidence={:.2}",
                q.question_id, q.x, q.y, q.width, q.height, q.confidence
            );
        }

        // 基本断言：应该检测到一些块（fallback 分割也应产生结果）
        assert!(r.question_count > 0, "Should detect at least 1 question region");
        println!("=== Test passed! ===");

        // 清理
        let _ = std::fs::remove_file(&temp_path);
        let _ = dispose_detector();
    }
}
