//! 核心数据类型定义
//!
//! 这些类型会通过 flutter_rust_bridge 自动生成对应的 Dart 类

use crate::geometry::Rect;
use serde::{Deserialize, Serialize};

/// 检测器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// 预处理配置
    pub preprocessing: PreprocessingConfig,
    /// 题号模式配置
    pub question_patterns: Vec<QuestionPattern>,
    /// OCR 配置（可选）
    pub ocr: Option<OcrConfig>,
    /// 版面分析配置（可选）
    pub layout: Option<LayoutConfig>,
    /// Debug 配置
    pub debug: DebugConfig,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            preprocessing: PreprocessingConfig::default(),
            question_patterns: QuestionPattern::default_patterns(),
            ocr: None,
            layout: None,
            debug: DebugConfig::default(),
        }
    }
}

/// 版面分析配置（DocLayout-YOLO）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// ONNX 模型文件路径
    pub model_path: Option<String>,
    /// 置信度阈值（默认 0.3）
    pub confidence_threshold: Option<f32>,
    /// 输入尺寸（默认 1024）
    pub input_size: Option<u32>,
}

/// 预处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    /// 启用倾斜校正
    pub enable_deskew: bool,
    /// 启用去噪
    pub enable_denoise: bool,
    /// 二值化方法
    pub binarization_method: BinarizationMethod,
    /// 对比度增强系数
    pub contrast_enhancement: f32,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            enable_deskew: true,
            enable_denoise: true,
            binarization_method: BinarizationMethod::Adaptive,
            contrast_enhancement: 1.2,
        }
    }
}

/// 二值化方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinarizationMethod {
    /// OTSU 算法
    Otsu,
    /// 自适应阈值
    Adaptive,
    /// 固定阈值
    Fixed,
}

/// 题号模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionPattern {
    /// 正则表达式模式
    pub pattern: String,
    /// 模式类型
    pub pattern_type: PatternType,
    /// 优先级（数字越大优先级越高）
    pub priority: u8,
}

impl QuestionPattern {
    /// 默认题号模式库
    pub fn default_patterns() -> Vec<Self> {
        vec![
            QuestionPattern {
                pattern: r"^\d+\.".to_string(),
                pattern_type: PatternType::Numbered,
                priority: 10,
            },
            QuestionPattern {
                pattern: r"^\(\d+\)".to_string(),
                pattern_type: PatternType::Parenthesized,
                priority: 9,
            },
            QuestionPattern {
                pattern: r"^[一二三四五六七八九十百]+、".to_string(),
                pattern_type: PatternType::Chinese,
                priority: 8,
            },
            QuestionPattern {
                pattern: r"^【\d+】".to_string(),
                pattern_type: PatternType::Bracketed,
                priority: 7,
            },
        ]
    }
}

/// 题号模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternType {
    /// 数字编号：1. 2. 3.
    Numbered,
    /// 括号编号：(1) (2) (3)
    Parenthesized,
    /// 中文编号：一、二、三、
    Chinese,
    /// 方括号编号：【1】【2】【3】
    Bracketed,
}

/// OCR 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    /// OCR 引擎类型
    pub engine: OcrEngine,
    /// 识别语言
    pub language: String,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 模型文件目录（PaddleOCR 引擎必填）
    /// 该目录下应包含:
    ///   det/inference.pdmodel + det/inference.pdiparams
    ///   rec/inference.pdmodel + rec/inference.pdiparams
    ///   ppocr_keys.txt
    pub model_dir: Option<String>,
}

/// OCR 引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OcrEngine {
    /// PaddleOCR (Paddle Inference C API，推荐)
    PaddleOCR,
    /// Tesseract OCR
    Tesseract,
    /// Mock OCR（用于测试）
    Mock,
}

/// Debug 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// 保存中间结果
    pub save_intermediate: bool,
    /// 输出目录
    pub output_dir: Option<String>,
    /// 详细日志
    pub verbose: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            save_intermediate: false,
            output_dir: None,
            verbose: false,
        }
    }
}

/// 检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// 检测状态
    pub status: DetectionStatus,
    /// 题目列表
    pub questions: Vec<QuestionBox>,
    /// 元数据
    pub metadata: ResultMetadata,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 检测状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionStatus {
    /// 成功
    Success,
    /// 部分成功（有些题目检测失败）
    PartialSuccess,
    /// 失败
    Failed,
}

/// 题目框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionBox {
    /// 页面索引
    pub page_index: usize,
    /// 题号
    pub question_id: String,
    /// 边界框
    pub bounding_box: Rect,
    /// 题号锚点框
    pub title_anchor_box: Option<Rect>,
    /// 置信度 (0.0-1.0)
    pub confidence: f64,
    /// 识别到的题号文本
    pub recognized_title_text: Option<String>,
    /// 包含的文本块 ID 列表
    pub block_ids: Vec<usize>,
    /// Debug 信息
    pub debug_info: Option<QuestionDebugInfo>,
}

/// 题目 Debug 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionDebugInfo {
    /// 包含的 block 数量
    pub num_blocks: usize,
    /// 检测方法
    pub detection_method: String,
    /// 是否有选项
    pub has_options: bool,
    /// 是否有图片
    pub has_image: bool,
}

/// 结果元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMetadata {
    /// 检测到的题目总数
    pub total_questions: usize,
    /// 处理时间（毫秒）
    pub processing_time_ms: u64,
    /// 图像宽度
    pub image_width: u32,
    /// 图像高度
    pub image_height: u32,
    /// 检测到的分栏数
    pub num_columns: usize,
}

/// 文本块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// 块 ID
    pub id: usize,
    /// 边界框
    pub bbox: Rect,
    /// 识别的文本（如果有）
    pub text: Option<String>,
    /// 置信度
    pub confidence: f64,
    /// 块类型
    pub block_type: BlockType,
}

/// 块类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockType {
    /// 文本
    Text,
    /// 题号
    QuestionNumber,
    /// 选项
    Option,
    /// 图片
    Image,
    /// 表格
    Table,
    /// 未知
    Unknown,
}

/// 题号锚点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnchor {
    /// 题号
    pub question_id: String,
    /// 位置
    pub bbox: Rect,
    /// 置信度
    pub confidence: f64,
    /// 识别的文本
    pub text: String,
    /// 模式类型
    pub pattern_type: PatternType,
}

/// 处理选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOptions {
    /// 页面索引（用于多页处理）
    pub page_index: usize,
    /// 是否返回 debug 信息
    pub include_debug: bool,
    /// 是否保存中间结果
    pub save_intermediate: bool,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            page_index: 0,
            include_debug: false,
            save_intermediate: false,
        }
    }
}
