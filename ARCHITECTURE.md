# 试卷题目识别与框选系统 - 架构设计文档

## 一、整体架构设计

### 1.1 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                    Flutter UI Layer                     │
│  - 图片显示与缩放                                         │
│  - 题目框 Overlay 绘制                                    │
│  - 用户交互（选题、修框）                                  │
│  - Debug 视图控制                                        │
└─────────────────────────────────────────────────────────┘
                           │ dart:ffi
                           ▼
┌─────────────────────────────────────────────────────────┐
│                   FFI Bridge Layer                      │
│  - C ABI 封装                                            │
│  - 内存管理（分配/释放）                                   │
│  - 数据序列化/反序列化                                     │
│  - 错误传递                                              │
└─────────────────────────────────────────────────────────┘
                           │ C ABI
                           ▼
┌─────────────────────────────────────────────────────────┐
│                  Rust Core Logic Layer                  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Image Preprocessing Module                   │  │
│  │  - 灰度化、去噪、二值化                            │  │
│  │  - 倾斜校正、透视变换                              │  │
│  │  - 对比度增强、边界检测                            │  │
│  └──────────────────────────────────────────────────┘  │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Block Detection Module                       │  │
│  │  - Connected Components 分析                      │  │
│  │  - 投影分析（水平/垂直）                           │  │
│  │  - 形态学操作                                     │  │
│  │  - Block 聚类与合并                               │  │
│  └──────────────────────────────────────────────────┘  │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │     OCR Adapter Layer                            │  │
│  │  - OCR Trait 定义                                 │  │
│  │  - Tesseract Adapter                             │  │
│  │  - 可扩展其他 OCR 引擎                            │  │
│  └──────────────────────────────────────────────────┘  │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Question Number Locator                      │  │
│  │  - 题号模式匹配（正则）                            │  │
│  │  - 题号位置定位                                   │  │
│  │  - 题号序列验证                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Question Segmentation Module                 │  │
│  │  - 基于题号的区域分段                              │  │
│  │  - Block 归属判定                                 │  │
│  │  - 多栏检测与处理                                  │  │
│  │  - 选项/图片吸附                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                         ▼                                │
│  ┌──────────────────────────────────────────────────┐  │
│  │     Confidence & Validation Module               │  │
│  │  - 分段结果置信度评分                              │  │
│  │  - 多假设生成与选择                                │  │
│  │  - 回退机制                                        │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 1.2 数据流

```
输入图片 (PNG/JPG/...)
    │
    ▼
[Rust] 图像预处理
    │
    ├─ 二值化图像
    ├─ 校正后图像
    └─ 增强图像
    │
    ▼
[Rust] Block 检测
    │
    └─ Block 列表 [{x,y,w,h}, ...]
    │
    ▼
[Rust] OCR 识别 (可选)
    │
    └─ 文本结果 [{text, box}, ...]
    │
    ▼
[Rust] 题号定位
    │
    └─ 题号锚点 [{question_id, box}, ...]
    │
    ▼
[Rust] 题目分段
    │
    └─ 题目框列表 [{question_id, box, confidence, blocks}, ...]
    │
    ▼
[FFI] 序列化为 JSON
    │
    ▼
[Flutter] 解析并渲染框选
```

### 1.3 责任边界

#### Flutter 层责任：
- UI 渲染与交互
- 图片加载与显示
- 框选结果可视化
- 用户手动修正框选
- 调用 Rust FFI
- 不包含任何识别算法

#### Rust 层责任：
- 所有图像处理算法
- 所有版面分析逻辑
- 所有题目分段逻辑
- OCR 引擎管理
- 置信度评估
- Debug 信息生成

#### FFI 层责任：
- 稳定的 C ABI 接口
- 内存安全管理
- 平台无关的数据传递
- 错误码定义与传递

## 二、FFI API 设计

### 2.1 核心 API

```rust
/// 初始化检测器
#[no_mangle]
pub extern "C" fn detector_create(config_json: *const c_char) -> *mut DetectorHandle;

/// 销毁检测器
#[no_mangle]
pub extern "C" fn detector_destroy(handle: *mut DetectorHandle);

/// 处理图片并返回题目框
/// 返回 JSON 字符串指针，需要调用 free_string 释放
#[no_mangle]
pub extern "C" fn detector_process_image(
    handle: *mut DetectorHandle,
    image_path: *const c_char,
    options_json: *const c_char,
) -> *mut c_char;

/// 批量处理多张图片
#[no_mangle]
pub extern "C" fn detector_process_batch(
    handle: *mut DetectorHandle,
    image_paths_json: *const c_char,
    options_json: *const c_char,
) -> *mut c_char;

/// 获取调试信息（中间处理结果）
#[no_mangle]
pub extern "C" fn detector_get_debug_info(
    handle: *mut DetectorHandle,
) -> *mut c_char;

/// 释放字符串
#[no_mangle]
pub extern "C" fn free_string(s: *mut c_char);

/// 获取最后一次错误信息
#[no_mangle]
pub extern "C" fn detector_last_error() -> *const c_char;
```

### 2.2 数据格式

#### 输入配置 (JSON)
```json
{
  "preprocessing": {
    "enable_deskew": true,
    "enable_denoise": true,
    "binarization_method": "adaptive",
    "contrast_enhancement": 1.2
  },
  "question_patterns": [
    {"pattern": "^\\d+\\.", "type": "numbered"},
    {"pattern": "^\\([\\d]+\\)", "type": "parenthesized"},
    {"pattern": "^[一二三四五六七八九十]+、", "type": "chinese"}
  ],
  "ocr": {
    "engine": "tesseract",
    "language": "chi_sim+eng"
  },
  "debug": {
    "save_intermediate": true,
    "output_dir": "/tmp/debug"
  }
}
```

#### 输出结果 (JSON)
```json
{
  "status": "success",
  "questions": [
    {
      "page_index": 0,
      "question_id": "1",
      "bounding_box": {
        "x": 100,
        "y": 200,
        "width": 500,
        "height": 150
      },
      "title_anchor_box": {
        "x": 100,
        "y": 200,
        "width": 30,
        "height": 20
      },
      "confidence": 0.95,
      "recognized_title_text": "1.",
      "block_ids": [1, 2, 3],
      "debug_info": {
        "num_blocks": 3,
        "detection_method": "anchor_based",
        "has_options": true
      }
    }
  ],
  "metadata": {
    "total_questions": 10,
    "processing_time_ms": 1250,
    "image_width": 2480,
    "image_height": 3508
  }
}
```

## 三、核心算法策略

### 3.1 图像预处理策略

**优先级**：去噪 > 二值化 > 倾斜校正 > 对比度增强

**关键点**：
- 使用自适应二值化（OTSU + Local Adaptive）
- Hough 变换检测倾斜角度
- 形态学操作去除噪点
- 保留原图供 OCR 使用

### 3.2 Block 检测策略

**不依赖 OCR 的版面分析**：
1. 投影分析：
   - 水平投影找行
   - 垂直投影找列
   - 识别空白区域作为分隔

2. Connected Components：
   - 检测连通区域
   - 过滤噪点（面积阈值）
   - 合并近邻区域

3. 几何规则：
   - 对齐关系（左对齐、缩进）
   - 垂直间距（行间距 vs 题间距）
   - 宽度一致性

### 3.3 题号定位策略

**多模式匹配**：
- 正则模式库（可配置）
- 位置约束（左侧、顶部）
- 序列连续性验证
- 支持跳号检测

**鲁棒性增强**：
- OCR 识别辅助
- 位置推断（等间距）
- 置信度阈值
- 回退到段落模式

### 3.4 题目分段策略

**基于题号的分段**：
1. 题号作为主 anchor
2. 从题号向下延伸到下一题号
3. 吸附同缩进 block
4. 吸附选项（A/B/C/D 模式）
5. 吸附图片区域（白色区域检测）

**多栏处理**：
1. 检测垂直分隔线
2. 投影分析找栏
3. 在每栏内独立分题
4. 按阅读顺序排序

**置信度评分**：
- 题号识别置信度 × 0.4
- Block 归属合理性 × 0.3
- 几何一致性 × 0.2
- OCR 辅助验证 × 0.1

## 四、扩展性设计

### 4.1 OCR 引擎可替换

```rust
pub trait OcrEngine {
    fn recognize(&self, image: &Image, config: &OcrConfig) -> Result<Vec<TextBlock>>;
    fn recognize_region(&self, image: &Image, region: Rect, config: &OcrConfig) -> Result<String>;
}

// 支持多种实现
impl OcrEngine for TesseractEngine { ... }
impl OcrEngine for PaddleOcrEngine { ... }
impl OcrEngine for CustomEngine { ... }
```

### 4.2 预处理管道可配置

```rust
pub struct PreprocessingPipeline {
    steps: Vec<Box<dyn PreprocessStep>>,
}

pub trait PreprocessStep {
    fn process(&self, image: &mut Image) -> Result<()>;
}
```

### 4.3 题号模式可扩展

```rust
pub struct QuestionPattern {
    pub regex: Regex,
    pub pattern_type: PatternType,
    pub priority: u8,
}

pub struct QuestionNumberLocator {
    patterns: Vec<QuestionPattern>,
}
```

## 五、Debug 与可观测性

### 5.1 中间结果导出

每个处理阶段都可导出：
- 二值化图像
- Block 检测框（绘制在图上）
- 题号位置标注
- 最终题目框
- 置信度热力图

### 5.2 日志系统

使用 `log` crate，支持多级别：
- ERROR：算法失败
- WARN：置信度低、回退
- INFO：处理进度
- DEBUG：详细算法参数
- TRACE：每个 block 的处理

### 5.3 性能监控

记录每个模块耗时：
- 预处理时间
- Block 检测时间
- OCR 时间
- 分段时间
- 总时间

## 六、平台适配

### 6.1 构建配置

```toml
[lib]
crate-type = ["cdylib", "staticlib"]

[target.'cfg(target_os = "android")']
# Android 特定配置

[target.'cfg(target_os = "ios")']
# iOS 特定配置

[target.'cfg(target_os = "windows")']
# Windows 特定配置
```

### 6.2 内存管理

- Rust 侧负责所有内存分配
- FFI 返回指针由 Rust 管理
- Flutter 调用 `free_string` 释放
- 使用 `Box::into_raw` 和 `Box::from_raw` 管理生命周期

### 6.3 错误处理

- Rust 不 panic，所有错误转为 Result
- FFI 层捕获 panic，返回错误码
- 错误信息通过 `detector_last_error()` 获取
- Flutter 层检查返回值并处理错误

## 七、设计权衡

### 7.1 为什么不依赖原生 OCR API？

**优点**：
- 完全跨平台，一次实现到处运行
- 算法可控，可持续优化
- 不受平台版本限制
- 离线可用

**代价**：
- 需要自行集成 OCR 引擎
- 初期识别率可能不如原生
- 包体积增大

**决策**：优先跨平台一致性，识别率可通过算法优化提升

### 7.2 为什么 OCR 只作为辅助？

**原因**：
- OCR 的版面分析不一定符合题目分段逻辑
- 纯依赖 OCR 鲁棒性差（模糊、噪点时失效）
- 几何分析更稳定
- OCR 主要用于：
  - 题号文本识别
  - 置信度验证
  - 难以从几何判断时的兜底

### 7.3 为什么使用 JSON 传递数据？

**优点**：
- 跨语言友好
- 易于调试
- 灵活扩展
- 人类可读

**代价**：
- 序列化开销
- 相比二进制格式体积大

**决策**：图像识别不是高频操作，可读性优先

### 7.4 为什么分模块这么细？

**原因**：
- 每个模块可独立测试
- 算法可替换
- 便于定位问题
- 支持渐进式优化

