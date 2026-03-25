//! PaddleOCR 引擎 — 基于 Paddle Inference C API
//!
//! 通过 FFI 调用 PaddlePaddle C++ 推理库，直接加载 Paddle 原生格式模型
//! （inference.pdmodel + inference.pdiparams），无需 ONNX 转换。
//!
//! 模型目录结构要求:
//! ```text
//! model_dir/
//! ├── det/
//! │   ├── inference.pdmodel
//! │   └── inference.pdiparams
//! ├── rec/
//! │   ├── inference.pdmodel
//! │   └── inference.pdiparams
//! └── ppocr_keys.txt
//! ```

use anyhow::{bail, Context};
use image::{DynamicImage, GenericImageView};
use std::fs;
use std::path::Path;

use crate::geometry::Rect;
use crate::ocr::{OcrEngineT, OcrResult};
use crate::paddle_ffi::*;

// ---------------------------------------------------------------------------
// Paddle Predictor 安全封装
// ---------------------------------------------------------------------------

/// 封装一个 Paddle Inference Predictor，提供 RAII 语义。
struct PaddlePredictor {
    predictor: *mut PD_Predictor,
}

impl PaddlePredictor {
    /// 从模型文件创建 Predictor
    fn new(model_file: &str, params_file: &str, num_threads: i32) -> anyhow::Result<Self> {
        unsafe {
            let config = PD_ConfigCreate();
            if config.is_null() {
                bail!("PD_ConfigCreate returned null");
            }

            let c_model = to_c_string(model_file);
            let c_params = to_c_string(params_file);
            PD_ConfigSetModel(config, c_model.as_ptr(), c_params.as_ptr());
            PD_ConfigDisableGpu(config);
            PD_ConfigSetCpuMathLibraryNumThreads(config, num_threads);
            PD_ConfigSwitchIrOptim(config, PD_TRUE);
            PD_ConfigEnableMemoryOptim(config, PD_TRUE);

            // 在 x86 平台启用 MKLDNN 加速
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                PD_ConfigEnableMKLDNN(config);
            }

            let predictor = PD_PredictorCreate(config);
            // Config 的所有权已转移给 Predictor，无需手动释放 config
            if predictor.is_null() {
                bail!(
                    "PD_PredictorCreate returned null. Model: {}, Params: {}",
                    model_file,
                    params_file
                );
            }

            Ok(Self { predictor })
        }
    }

    /// 获取输入 Tensor 名称列表
    fn input_names(&self) -> Vec<String> {
        unsafe { extract_all_names(PD_PredictorGetInputNames(self.predictor)) }
    }

    /// 获取输出 Tensor 名称列表
    fn output_names(&self) -> Vec<String> {
        unsafe { extract_all_names(PD_PredictorGetOutputNames(self.predictor)) }
    }

    /// 获取输入 Tensor 句柄
    fn input_handle(&self, name: &str) -> *mut PD_Tensor {
        let c_name = to_c_string(name);
        unsafe { PD_PredictorGetInputHandle(self.predictor, c_name.as_ptr()) }
    }

    /// 获取输出 Tensor 句柄
    fn output_handle(&self, name: &str) -> *mut PD_Tensor {
        let c_name = to_c_string(name);
        unsafe { PD_PredictorGetOutputHandle(self.predictor, c_name.as_ptr()) }
    }

    /// 执行推理
    fn run(&self) -> anyhow::Result<()> {
        let ok = unsafe { PD_PredictorRun(self.predictor) };
        if ok == PD_FALSE {
            bail!("PD_PredictorRun failed");
        }
        Ok(())
    }
}

impl Drop for PaddlePredictor {
    fn drop(&mut self) {
        if !self.predictor.is_null() {
            unsafe {
                PD_PredictorDestroy(self.predictor);
            }
        }
    }
}

// 推理过程本身不持有可变状态(通过 C API 内部管理)
// Paddle Inference Predictor 在同一个 predictor 实例上不支持并发 Run，
// 但我们通过 OcrAdapter 的 Mutex 保证了单线程访问。
unsafe impl Send for PaddlePredictor {}
unsafe impl Sync for PaddlePredictor {}

// ---------------------------------------------------------------------------
// DB 文字检测后处理
// ---------------------------------------------------------------------------

/// DB (Differentiable Binarization) 模型的后处理
struct DbPostProcessor {
    /// 二值化阈值
    thresh: f32,
    /// 最小文本框面积
    min_area: f64,
    /// 边框膨胀比例
    unclip_ratio: f64,
}

impl DbPostProcessor {
    fn new() -> Self {
        Self {
            thresh: 0.3,
            min_area: 16.0,
            unclip_ratio: 1.6,
        }
    }

    /// 从 DB 模型输出的概率图中提取文本框
    fn process(&self, prob_map: &[f32], height: usize, width: usize) -> Vec<TextBox> {
        // 二值化概率图
        let mut binary = vec![0u8; height * width];
        for y in 0..height {
            for x in 0..width {
                let p = prob_map[y * width + x];
                if p > self.thresh {
                    binary[y * width + x] = 255;
                }
            }
        }

        // 使用行投影法分割文本行，再用列投影法分割行内文本框
        let raw_boxes = self.find_text_boxes_by_projection(&binary, prob_map, height, width);

        let mut boxes: Vec<TextBox> = raw_boxes
            .into_iter()
            .filter(|b| {
                // 过滤面积过小的框
                if b.rect.width * b.rect.height < self.min_area {
                    return false;
                }
                // 过滤高度 < 8px 的框（太小不可能是有意义的文本）
                if b.rect.height < 8.0 {
                    return false;
                }
                true
            })
            .collect();

        // NMS 去除重叠框
        self.nms(&mut boxes, 0.3);

        // 按 y 坐标排序，然后 x
        boxes.sort_by(|a, b| {
            let dy = a.rect.y.partial_cmp(&b.rect.y).unwrap();
            if dy == std::cmp::Ordering::Equal {
                a.rect.x.partial_cmp(&b.rect.x).unwrap()
            } else {
                dy
            }
        });

        boxes
    }

    /// 使用行投影和列投影法提取文本框
    ///
    /// 1. 计算水平投影（每行前景像素数），找到文本行带
    /// 2. 对每个文本行带，计算垂直投影，找到行内各文本框
    fn find_text_boxes_by_projection(
        &self,
        binary: &[u8],
        prob_map: &[f32],
        height: usize,
        width: usize,
    ) -> Vec<TextBox> {
        let mut boxes = Vec::new();

        // Step 1: 水平投影 — 每行的前景像素计数
        let mut h_proj = vec![0u32; height];
        for y in 0..height {
            for x in 0..width {
                if binary[y * width + x] > 0 {
                    h_proj[y] += 1;
                }
            }
        }

        // Step 2: 找到连续的文本行带（投影 > 0 的行范围）
        let row_bands = Self::find_nonzero_ranges(&h_proj);

        for (row_start, row_end) in row_bands {
            // Step 3: 对该行带计算垂直投影
            let mut v_proj = vec![0u32; width];
            for y in row_start..row_end {
                for x in 0..width {
                    if binary[y * width + x] > 0 {
                        v_proj[x] += 1;
                    }
                }
            }

            // Step 4: 找到列范围（投影 > 0 的列范围）
            let col_bands = Self::find_nonzero_ranges(&v_proj);

            for (col_start, col_end) in col_bands {
                let raw_w = (col_end - col_start) as f64;
                let raw_h = (row_end - row_start) as f64;

                // 膨胀边框
                let pad_w = raw_w * (self.unclip_ratio - 1.0) / 2.0;
                let pad_h = raw_h * (self.unclip_ratio - 1.0) / 2.0;
                let bx = (col_start as f64 - pad_w).max(0.0);
                let by = (row_start as f64 - pad_h).max(0.0);
                let bw = (raw_w + 2.0 * pad_w).min(width as f64 - bx);
                let bh = (raw_h + 2.0 * pad_h).min(height as f64 - by);

                // 计算该区域内的平均概率作为置信度
                let mut sum = 0.0f64;
                let mut count = 0u32;
                let x0 = bx as usize;
                let y0 = by as usize;
                let x1 = (bx + bw) as usize;
                let y1 = (by + bh) as usize;
                for py in y0..y1.min(height) {
                    for px in x0..x1.min(width) {
                        sum += prob_map[py * width + px] as f64;
                        count += 1;
                    }
                }
                let confidence = if count > 0 { sum / count as f64 } else { 0.0 };

                boxes.push(TextBox {
                    rect: Rect::new(bx, by, bw, bh),
                    confidence,
                });
            }
        }

        boxes
    }

    /// 在投影数组中找到连续非零区间
    ///
    /// 返回 (start, end) 对的列表，其中 end 是排他的。
    fn find_nonzero_ranges(proj: &[u32]) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut i = 0;
        while i < proj.len() {
            if proj[i] > 0 {
                let start = i;
                while i < proj.len() && proj[i] > 0 {
                    i += 1;
                }
                ranges.push((start, i));
            } else {
                i += 1;
            }
        }
        ranges
    }

    /// 非极大值抑制（NMS）
    ///
    /// 按置信度降序排列，依次保留高置信度框，去除与其 IoU 超过阈值的低置信度框。
    fn nms(&self, boxes: &mut Vec<TextBox>, iou_threshold: f64) {
        // 按置信度降序排序
        boxes.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let mut keep = vec![true; boxes.len()];
        for i in 0..boxes.len() {
            if !keep[i] {
                continue;
            }
            for j in (i + 1)..boxes.len() {
                if !keep[j] {
                    continue;
                }
                if Self::compute_iou(&boxes[i].rect, &boxes[j].rect) > iou_threshold {
                    keep[j] = false;
                }
            }
        }

        let mut idx = 0;
        boxes.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }

    /// 计算两个矩形的 IoU (Intersection over Union)
    fn compute_iou(a: &Rect, b: &Rect) -> f64 {
        let x1 = a.x.max(b.x);
        let y1 = a.y.max(b.y);
        let x2 = (a.x + a.width).min(b.x + b.width);
        let y2 = (a.y + a.height).min(b.y + b.height);

        if x2 <= x1 || y2 <= y1 {
            return 0.0;
        }

        let inter = (x2 - x1) * (y2 - y1);
        let area_a = a.width * a.height;
        let area_b = b.width * b.height;
        let union = area_a + area_b - inter;

        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }
}

/// 检测到的文本框
#[derive(Debug, Clone)]
struct TextBox {
    rect: Rect,
    confidence: f64,
}

// ---------------------------------------------------------------------------
// CRNN 文字识别后处理
// ---------------------------------------------------------------------------

/// CTC 贪心解码器
struct CtcDecoder {
    keys: Vec<String>,
    confidence_threshold: f32,
}

impl CtcDecoder {
    fn new(keys: Vec<String>, confidence_threshold: f32) -> Self {
        Self {
            keys,
            confidence_threshold,
        }
    }

    /// 贪心 CTC 解码
    fn decode(&self, logits: &[f32], timesteps: usize, classes: usize) -> (String, f32) {
        let mut last = 0usize;
        let mut text = String::new();
        let mut probs = Vec::new();

        for t in 0..timesteps {
            let offset = t * classes;
            let step = &logits[offset..offset + classes];

            // softmax 取最大值
            let max_val = step.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let exp_sum: f32 = step.iter().map(|&v| (v - max_val).exp()).sum();

            if let Some((idx, &val)) = step
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            {
                let prob = (val - max_val).exp() / exp_sum;
                if idx != 0 && idx != last {
                    if let Some(ch) = self.keys.get(idx) {
                        text.push_str(ch);
                        probs.push(prob);
                    }
                }
                last = idx;
            }
        }

        let avg_prob = if probs.is_empty() {
            0.0
        } else {
            probs.iter().sum::<f32>() / probs.len() as f32
        };

        (text, avg_prob)
    }
}

// ---------------------------------------------------------------------------
// PaddleOCR 引擎
// ---------------------------------------------------------------------------

/// 基于 Paddle Inference C API 的 PaddleOCR 引擎
///
/// 支持完整的 检测(det) + 识别(rec) pipeline：
/// 1. det 模型检测文本行位置
/// 2. 裁剪文本行图片
/// 3. rec 模型识别每行文字
pub struct PaddleOcrEngine {
    /// 文字检测 Predictor
    det_predictor: PaddlePredictor,
    /// 文字识别 Predictor
    rec_predictor: PaddlePredictor,
    /// CTC 解码器
    ctc_decoder: CtcDecoder,
    /// DB 后处理器
    db_processor: DbPostProcessor,
    /// det 模型的输入尺寸（长边最大值）
    det_max_side: u32,
    /// rec 模型的输入高度
    rec_img_height: u32,
}

impl PaddleOcrEngine {
    /// 创建 PaddleOCR 引擎
    ///
    /// # 参数
    /// - `model_dir`: 模型目录，结构如下:
    ///   ```text
    ///   model_dir/
    ///   ├── det/inference.pdmodel + inference.pdiparams
    ///   ├── rec/inference.pdmodel + inference.pdiparams
    ///   └── ppocr_keys.txt
    ///   ```
    /// - `confidence_threshold`: 最低置信度阈值
    pub fn new(model_dir: &str, confidence_threshold: f32) -> anyhow::Result<Self> {
        let dir = Path::new(model_dir);

        // 验证模型文件
        let det_model = dir.join("det").join("inference.pdmodel");
        let det_params = dir.join("det").join("inference.pdiparams");
        let rec_model = dir.join("rec").join("inference.pdmodel");
        let rec_params = dir.join("rec").join("inference.pdiparams");
        let keys_path = dir.join("ppocr_keys.txt");

        for p in [&det_model, &det_params, &rec_model, &rec_params, &keys_path] {
            anyhow::ensure!(p.exists(), "Missing required file: {}", p.display());
        }

        // 加载字符字典
        let keys_raw = fs::read_to_string(&keys_path)
            .with_context(|| format!("Failed to read {}", keys_path.display()))?;
        let mut keys: Vec<String> = keys_raw
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        // PP-OCR 字典第 0 位是 blank token
        keys.insert(0, String::new());
        // 末尾补一个空白（某些模型最后一个 class 是 blank）
        keys.push(String::new());

        log::info!(
            "PaddleOCR: loaded {} character keys from {}",
            keys.len(),
            keys_path.display()
        );

        // 创建 Predictor
        let det_model_str = det_model.to_str().unwrap();
        let det_params_str = det_params.to_str().unwrap();
        let rec_model_str = rec_model.to_str().unwrap();
        let rec_params_str = rec_params.to_str().unwrap();

        let num_threads = num_cpus().min(4) as i32;

        let det_predictor = PaddlePredictor::new(det_model_str, det_params_str, num_threads)
            .context("Failed to create detection predictor")?;

        let rec_predictor = PaddlePredictor::new(rec_model_str, rec_params_str, num_threads)
            .context("Failed to create recognition predictor")?;

        log::info!("PaddleOCR: det input names: {:?}", det_predictor.input_names());
        log::info!("PaddleOCR: rec input names: {:?}", rec_predictor.input_names());

        let ctc_decoder = CtcDecoder::new(keys, confidence_threshold);
        let db_processor = DbPostProcessor::new();

        Ok(Self {
            det_predictor,
            rec_predictor,
            ctc_decoder,
            db_processor,
            det_max_side: 960,
            rec_img_height: 48,
        })
    }

    /// 预处理图片用于 det 模型
    ///
    /// PP-OCRv4 det 输入: [1, 3, H, W]，归一化到 [0,1] 后标准化
    fn preprocess_det(&self, image: &DynamicImage) -> (Vec<f32>, u32, u32, f64) {
        let (orig_w, orig_h) = image.dimensions();

        // 长边缩放到 det_max_side，且保持 32 的倍数
        let ratio = if orig_w.max(orig_h) > self.det_max_side {
            self.det_max_side as f64 / orig_w.max(orig_h) as f64
        } else {
            1.0
        };

        let new_w = ((orig_w as f64 * ratio) as u32).max(32);
        let new_h = ((orig_h as f64 * ratio) as u32).max(32);
        // 对齐到 32 的倍数
        let new_w = (new_w + 31) / 32 * 32;
        let new_h = (new_h + 31) / 32 * 32;

        let resized = image.resize_exact(new_w, new_h, image::imageops::FilterType::CatmullRom);
        let rgb = resized.to_rgb8();

        // 标准化: (pixel / 255.0 - mean) / std
        // PP-OCR 使用 mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
        let mean = [0.485f32, 0.456, 0.406];
        let std = [0.229f32, 0.224, 0.225];

        let h = new_h as usize;
        let w = new_w as usize;
        let mut data = vec![0.0f32; 3 * h * w];
        for c in 0..3usize {
            for y in 0..h {
                for x in 0..w {
                    let pixel = rgb.get_pixel(x as u32, y as u32)[c] as f32 / 255.0;
                    let idx = c * h * w + y * w + x;
                    data[idx] = (pixel - mean[c]) / std[c];
                }
            }
        }

        (data, new_w, new_h, ratio)
    }

    /// 预处理裁剪的文本行用于 rec 模型
    ///
    /// PP-OCRv4 rec 输入: [1, 3, 48, W]，保持宽高比
    fn preprocess_rec(&self, image: &DynamicImage) -> (Vec<f32>, u32) {
        let (w, h) = image.dimensions();
        let target_h = self.rec_img_height;
        let ratio = target_h as f32 / h as f32;
        let target_w = ((w as f32 * ratio).round() as u32).clamp(16, 320);

        let resized =
            image.resize_exact(target_w, target_h, image::imageops::FilterType::CatmullRom);
        let rgb = resized.to_rgb8();

        // 归一化: (pixel / 255.0 - 0.5) / 0.5
        let mut data = vec![0.0f32; (3 * target_h * target_w) as usize];
        for c in 0..3usize {
            for y in 0..target_h {
                for x in 0..target_w {
                    let v = rgb.get_pixel(x, y)[c as usize] as f32 / 255.0;
                    let idx = c * (target_h * target_w) as usize + (y * target_w + x) as usize;
                    data[idx] = (v - 0.5) / 0.5;
                }
            }
        }

        (data, target_w)
    }

    /// 运行文字检测模型
    fn run_det(&self, image: &DynamicImage) -> anyhow::Result<Vec<TextBox>> {
        let (data, det_w, det_h, _ratio) = self.preprocess_det(image);

        let input_names = self.det_predictor.input_names();
        let output_names = self.det_predictor.output_names();

        if input_names.is_empty() || output_names.is_empty() {
            bail!("Det model has no input/output names");
        }

        unsafe {
            // 设置输入
            let input = self.det_predictor.input_handle(&input_names[0]);
            let shape = [1i32, 3, det_h as i32, det_w as i32];
            PD_TensorReshape(input, 4, shape.as_ptr());
            PD_TensorCopyFromCpuFloat(input, data.as_ptr());

            // 执行推理
            self.det_predictor.run()?;

            // 获取输出
            let output = self.det_predictor.output_handle(&output_names[0]);
            let out_shape = extract_shape(PD_TensorGetShape(output));

            if out_shape.len() < 3 {
                bail!("Unexpected det output shape: {:?}", out_shape);
            }

            let out_h = out_shape[out_shape.len() - 2] as usize;
            let out_w = out_shape[out_shape.len() - 1] as usize;
            let total = out_shape.iter().map(|&x| x as usize).product::<usize>();
            let mut out_data = vec![0.0f32; total];
            PD_TensorCopyToCpuFloat(output, out_data.as_mut_ptr());

            // 如果输出是 [1, 1, H, W]，取最后 H*W
            let prob_map = if out_data.len() > out_h * out_w {
                &out_data[out_data.len() - out_h * out_w..]
            } else {
                &out_data
            };

            // DB 后处理
            let mut boxes = self.db_processor.process(prob_map, out_h, out_w);

            // 将坐标映射回原图
            let (orig_w, orig_h) = image.dimensions();
            let scale_x = orig_w as f64 / det_w as f64;
            let scale_y = orig_h as f64 / det_h as f64;

            for b in &mut boxes {
                b.rect.x *= scale_x;
                b.rect.y *= scale_y;
                b.rect.width *= scale_x;
                b.rect.height *= scale_y;

                // 裁剪到图像范围内
                b.rect.x = b.rect.x.max(0.0);
                b.rect.y = b.rect.y.max(0.0);
                if b.rect.x + b.rect.width > orig_w as f64 {
                    b.rect.width = orig_w as f64 - b.rect.x;
                }
                if b.rect.y + b.rect.height > orig_h as f64 {
                    b.rect.height = orig_h as f64 - b.rect.y;
                }
            }

            Ok(boxes)
        }
    }

    /// 运行文字识别模型（识别单行文本）
    fn run_rec(&self, line_image: &DynamicImage) -> anyhow::Result<(String, f32)> {
        let (data, target_w) = self.preprocess_rec(line_image);

        let input_names = self.rec_predictor.input_names();
        let output_names = self.rec_predictor.output_names();

        if input_names.is_empty() || output_names.is_empty() {
            bail!("Rec model has no input/output names");
        }

        unsafe {
            // 设置输入
            let input = self.rec_predictor.input_handle(&input_names[0]);
            let shape = [1i32, 3, self.rec_img_height as i32, target_w as i32];
            PD_TensorReshape(input, 4, shape.as_ptr());
            PD_TensorCopyFromCpuFloat(input, data.as_ptr());

            // 执行推理
            self.rec_predictor.run()?;

            // 获取输出 [1, T, C]
            let output = self.rec_predictor.output_handle(&output_names[0]);
            let out_shape = extract_shape(PD_TensorGetShape(output));

            if out_shape.len() != 3 {
                bail!("Unexpected rec output shape: {:?}", out_shape);
            }

            let timesteps = out_shape[1] as usize;
            let classes = out_shape[2] as usize;
            let total = (out_shape[0] * out_shape[1] * out_shape[2]) as usize;
            let mut out_data = vec![0.0f32; total];
            PD_TensorCopyToCpuFloat(output, out_data.as_mut_ptr());

            // CTC 解码
            Ok(self.ctc_decoder.decode(&out_data, timesteps, classes))
        }
    }
}

impl OcrEngineT for PaddleOcrEngine {
    fn recognize(&self, image: &DynamicImage) -> anyhow::Result<Vec<OcrResult>> {
        // Step 1: 检测文本行
        let text_boxes = self.run_det(image)?;

        if text_boxes.is_empty() {
            log::debug!("PaddleOCR: no text boxes detected");
            return Ok(vec![]);
        }

        log::debug!("PaddleOCR: detected {} text boxes", text_boxes.len());

        // Step 2: 识别每个文本行
        let mut results = Vec::new();
        for tb in &text_boxes {
            // 裁剪文本行
            let x = tb.rect.x.max(0.0) as u32;
            let y = tb.rect.y.max(0.0) as u32;
            let w = (tb.rect.width as u32).max(1);
            let h = (tb.rect.height as u32).max(1);

            // 防止超出图像边界
            let (img_w, img_h) = image.dimensions();
            if x >= img_w || y >= img_h {
                continue;
            }
            let w = w.min(img_w - x);
            let h = h.min(img_h - y);
            if w < 2 || h < 2 {
                continue;
            }

            let cropped = image.crop_imm(x, y, w, h);
            let (text, prob) = self.run_rec(&cropped)?;

            if text.is_empty() || prob < self.ctc_decoder.confidence_threshold {
                continue;
            }

            results.push(OcrResult {
                text,
                bbox: tb.rect,
                confidence: prob as f64,
            });
        }

        Ok(results)
    }

    fn recognize_region(&self, image: &DynamicImage, region: Rect) -> anyhow::Result<String> {
        let x = region.x.max(0.0) as u32;
        let y = region.y.max(0.0) as u32;
        let w = (region.width as u32).max(1);
        let h = (region.height as u32).max(1);

        let (img_w, img_h) = image.dimensions();
        if x >= img_w || y >= img_h {
            return Ok(String::new());
        }
        let w = w.min(img_w - x);
        let h = h.min(img_h - y);

        let cropped = image.crop_imm(x, y, w, h);

        // 对于指定区域，直接运行 rec（假设区域内只有一行文本）
        let (text, prob) = self.run_rec(&cropped)?;
        if prob < self.ctc_decoder.confidence_threshold {
            return Ok(String::new());
        }
        Ok(text)
    }
}

/// 获取 CPU 核心数
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}
