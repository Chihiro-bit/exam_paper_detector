//! 通用工具模块
//!
//! 提供所有模块共用的基础类型和工具函数，包括：
//! - 几何类型（Rect, Point, Size）
//! - 错误定义
//! - 日志工具
//! - Debug 工具

pub mod geometry;
pub mod error;
pub mod logger;
pub mod debug;

pub use geometry::{Point, Rect, Size};
pub use error::{Error, Result};
