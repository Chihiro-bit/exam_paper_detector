//! Paddle Inference C API 的 Rust FFI 绑定
//!
//! 基于 PaddlePaddle Inference C API (paddle_c_api.h) 的手动绑定。
//! 支持 Paddle Inference 2.5+ 版本。
//!
//! 使用方需要预先下载 Paddle Inference C 库并设置环境变量
//! `PADDLE_INFERENCE_DIR` 指向解压后的目录。

#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::os::raw::{c_char, c_float, c_int};

// ---------------------------------------------------------------------------
// 基础类型
// ---------------------------------------------------------------------------

pub type PD_Bool = i8;
pub const PD_TRUE: PD_Bool = 1;
pub const PD_FALSE: PD_Bool = 0;

/// Paddle 数据类型枚举
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PD_DataType {
    PD_DATA_FLOAT32 = 0,
    PD_DATA_INT32 = 1,
    PD_DATA_INT64 = 2,
    PD_DATA_UINT8 = 3,
    PD_DATA_INT8 = 4,
}

// ---------------------------------------------------------------------------
// 一维数组类型（C API 返回值）
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct PD_OneDimArrayCstr {
    pub size: usize,
    pub data: *mut *mut c_char,
}

#[repr(C)]
pub struct PD_OneDimArrayInt32 {
    pub size: usize,
    pub data: *mut i32,
}

#[repr(C)]
pub struct PD_OneDimArraySize {
    pub size: usize,
    pub data: *mut usize,
}

// ---------------------------------------------------------------------------
// 不透明句柄类型
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct PD_Config {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct PD_Predictor {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct PD_Tensor {
    _opaque: [u8; 0],
}

// ---------------------------------------------------------------------------
// 外部函数声明
// ---------------------------------------------------------------------------

extern "C" {
    // -- 一维数组销毁 --
    pub fn PD_OneDimArrayCstrDestroy(array: *mut PD_OneDimArrayCstr);
    pub fn PD_OneDimArrayInt32Destroy(array: *mut PD_OneDimArrayInt32);

    // -- Config --
    pub fn PD_ConfigCreate() -> *mut PD_Config;
    pub fn PD_ConfigDestroy(config: *mut PD_Config);

    /// 设置模型文件路径 (combined model: .pdmodel + .pdiparams)
    pub fn PD_ConfigSetModel(
        config: *mut PD_Config,
        prog_file: *const c_char,
        params_file: *const c_char,
    );

    /// 设置模型目录 (separated model: __model__ + __params__)
    pub fn PD_ConfigSetModelDir(config: *mut PD_Config, model_dir: *const c_char);

    /// 禁用 GPU
    pub fn PD_ConfigDisableGpu(config: *mut PD_Config);

    /// 设置 CPU 数学库线程数
    pub fn PD_ConfigSetCpuMathLibraryNumThreads(config: *mut PD_Config, threads: c_int);

    /// 启用/禁用 IR 优化
    pub fn PD_ConfigSwitchIrOptim(config: *mut PD_Config, enable: PD_Bool);

    /// 启用内存优化
    pub fn PD_ConfigEnableMemoryOptim(config: *mut PD_Config, enable: PD_Bool);

    /// 启用 MKLDNN 加速 (仅 x86)
    pub fn PD_ConfigEnableMKLDNN(config: *mut PD_Config);

    /// 删除传递的优化 pass
    pub fn PD_ConfigDeletePass(config: *mut PD_Config, pass_name: *const c_char);

    // -- Predictor --
    pub fn PD_PredictorCreate(config: *const PD_Config) -> *mut PD_Predictor;
    pub fn PD_PredictorDestroy(predictor: *mut PD_Predictor);

    /// 执行推理
    pub fn PD_PredictorRun(predictor: *mut PD_Predictor) -> PD_Bool;

    /// 获取输入名称列表
    pub fn PD_PredictorGetInputNames(predictor: *mut PD_Predictor) -> *mut PD_OneDimArrayCstr;

    /// 获取输出名称列表
    pub fn PD_PredictorGetOutputNames(predictor: *mut PD_Predictor) -> *mut PD_OneDimArrayCstr;

    /// 获取输入 Tensor 句柄（按名称）
    pub fn PD_PredictorGetInputHandle(
        predictor: *mut PD_Predictor,
        name: *const c_char,
    ) -> *mut PD_Tensor;

    /// 获取输出 Tensor 句柄（按名称）
    pub fn PD_PredictorGetOutputHandle(
        predictor: *mut PD_Predictor,
        name: *const c_char,
    ) -> *mut PD_Tensor;

    // -- Tensor --
    pub fn PD_TensorDestroy(tensor: *mut PD_Tensor);

    /// 重设 Tensor 形状
    pub fn PD_TensorReshape(tensor: *mut PD_Tensor, ndim: i32, shape: *const i32);

    /// 从 CPU 拷贝 float 数据到 Tensor
    pub fn PD_TensorCopyFromCpuFloat(tensor: *mut PD_Tensor, data: *const c_float);

    /// 从 CPU 拷贝 int32 数据到 Tensor
    pub fn PD_TensorCopyFromCpuInt32(tensor: *mut PD_Tensor, data: *const i32);

    /// 从 CPU 拷贝 int64 数据到 Tensor
    pub fn PD_TensorCopyFromCpuInt64(tensor: *mut PD_Tensor, data: *const i64);

    /// 从 Tensor 拷贝 float 数据到 CPU
    pub fn PD_TensorCopyToCpuFloat(tensor: *mut PD_Tensor, data: *mut c_float);

    /// 从 Tensor 拷贝 int32 数据到 CPU
    pub fn PD_TensorCopyToCpuInt32(tensor: *mut PD_Tensor, data: *mut i32);

    /// 从 Tensor 拷贝 int64 数据到 CPU
    pub fn PD_TensorCopyToCpuInt64(tensor: *mut PD_Tensor, data: *mut i64);

    /// 获取 Tensor 形状
    pub fn PD_TensorGetShape(tensor: *mut PD_Tensor) -> *mut PD_OneDimArrayInt32;

    /// 获取 Tensor 名称
    pub fn PD_TensorGetName(tensor: *mut PD_Tensor) -> *const c_char;

    /// 获取 Tensor 数据类型
    pub fn PD_TensorGetDataType(tensor: *mut PD_Tensor) -> PD_DataType;
}

// ---------------------------------------------------------------------------
// 安全包装
// ---------------------------------------------------------------------------

/// 安全地创建 C 字符串（以 null 结尾）
pub fn to_c_string(s: &str) -> std::ffi::CString {
    std::ffi::CString::new(s).expect("String contains null byte")
}

/// 从 PD_OneDimArrayCstr 提取第一个字符串名称（用于获取输入/输出名称）
///
/// # Safety
/// 调用者确保 `array` 有效且尚未被释放
pub unsafe fn extract_first_name(array: *mut PD_OneDimArrayCstr) -> Option<String> {
    if array.is_null() {
        return None;
    }
    let arr = &*array;
    if arr.size == 0 || arr.data.is_null() {
        PD_OneDimArrayCstrDestroy(array);
        return None;
    }
    let first = *arr.data;
    let name = if first.is_null() {
        None
    } else {
        Some(std::ffi::CStr::from_ptr(first).to_string_lossy().into_owned())
    };
    PD_OneDimArrayCstrDestroy(array);
    name
}

/// 从 PD_OneDimArrayCstr 提取所有字符串名称
///
/// # Safety
/// 调用者确保 `array` 有效且尚未被释放
pub unsafe fn extract_all_names(array: *mut PD_OneDimArrayCstr) -> Vec<String> {
    if array.is_null() {
        return vec![];
    }
    let arr = &*array;
    let mut names = Vec::with_capacity(arr.size);
    for i in 0..arr.size {
        let ptr = *arr.data.add(i);
        if !ptr.is_null() {
            names.push(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned());
        }
    }
    PD_OneDimArrayCstrDestroy(array);
    names
}

/// 从 PD_OneDimArrayInt32 提取形状
///
/// # Safety
/// 调用者确保 `array` 有效且尚未被释放
pub unsafe fn extract_shape(array: *mut PD_OneDimArrayInt32) -> Vec<i32> {
    if array.is_null() {
        return vec![];
    }
    let arr = &*array;
    let mut shape = Vec::with_capacity(arr.size);
    for i in 0..arr.size {
        shape.push(*arr.data.add(i));
    }
    PD_OneDimArrayInt32Destroy(array);
    shape
}
