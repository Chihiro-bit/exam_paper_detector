//! 试卷题目识别与框选系统 - Rust 核心库
//!
//! 使用 flutter_rust_bridge 暴露 API 给 Flutter

mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */

mod api;
mod types;
mod geometry;
mod preprocessing;
mod block_detection;
mod ocr;
mod paddle_ffi;
mod ocr_paddle;
mod layout_detector;
mod question_locator;
mod segmentation;
mod detector;

// 重新导出公共 API
pub use api::*;
