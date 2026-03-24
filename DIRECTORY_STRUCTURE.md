# 项目目录结构

## 总体结构

```
exam_paper_detector/
├── ARCHITECTURE.md              # 架构设计文档
├── DIRECTORY_STRUCTURE.md       # 本文件
├── MVP_ROADMAP.md              # MVP 开发路线图
├── README.md                   # 项目说明
│
├── flutter_app/                # Flutter 应用
│   ├── pubspec.yaml
│   ├── lib/
│   │   ├── main.dart
│   │   ├── ffi/               # FFI 绑定层
│   │   │   ├── detector_ffi.dart          # FFI 接口定义
│   │   │   ├── detector_bindings.dart     # 自动生成的绑定
│   │   │   └── native_library.dart        # 动态库加载
│   │   ├── models/            # 数据模型
│   │   │   ├── question_box.dart          # 题目框模型
│   │   │   ├── detection_result.dart      # 检测结果
│   │   │   ├── detector_config.dart       # 配置模型
│   │   │   └── debug_info.dart            # Debug 信息
│   │   ├── services/          # 业务逻辑
│   │   │   └── detector_service.dart      # 检测服务
│   │   ├── widgets/           # UI 组件
│   │   │   ├── image_viewer.dart          # 图片查看器
│   │   │   ├── question_overlay.dart      # 题目框叠加层
│   │   │   └── debug_panel.dart           # Debug 面板
│   │   └── screens/           # 页面
│   │       ├── home_screen.dart           # 主页
│   │       └── detection_screen.dart      # 检测页面
│   ├── assets/                # 资源文件
│   │   └── test_images/       # 测试图片
│   └── test/                  # 测试
│       └── widget_test.dart
│
└── rust_core/                 # Rust 核心库
    ├── Cargo.toml             # Workspace 配置
    ├── build_all.sh           # 全平台编译脚本
    ├── build_all.bat          # Windows 编译脚本
    │
    ├── detector_ffi/          # FFI 导出层
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs         # FFI 主入口
    │   │   ├── api.rs         # C API 定义
    │   │   ├── handle.rs      # Handle 管理
    │   │   └── error.rs       # 错误处理
    │   └── cbindgen.toml      # C 头文件生成配置
    │
    ├── detector_core/         # 核心检测逻辑
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── detector.rs    # 检测器主接口
    │   │   ├── config.rs      # 配置定义
    │   │   └── pipeline.rs    # 处理管道
    │   └── tests/
    │       └── integration_test.rs
    │
    ├── image_processing/      # 图像处理模块
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── types.rs       # 图像类型定义
    │   │   ├── preprocessing/
    │   │   │   ├── mod.rs
    │   │   │   ├── grayscale.rs      # 灰度化
    │   │   │   ├── denoise.rs        # 去噪
    │   │   │   ├── binarization.rs   # 二值化
    │   │   │   ├── deskew.rs         # 倾斜校正
    │   │   │   └── enhance.rs        # 增强
    │   │   └── utils.rs       # 工具函数
    │   └── tests/
    │       └── preprocessing_test.rs
    │
    ├── block_detection/       # 区块检测模块
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── types.rs       # Block 类型定义
    │   │   ├── connected_components.rs  # 连通域分析
    │   │   ├── projection.rs            # 投影分析
    │   │   ├── morphology.rs            # 形态学操作
    │   │   ├── clustering.rs            # 聚类合并
    │   │   └── column_detector.rs       # 分栏检测
    │   └── tests/
    │       └── block_detection_test.rs
    │
    ├── ocr_adapter/           # OCR 适配层
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── traits.rs      # OCR Trait 定义
    │   │   ├── types.rs       # OCR 类型
    │   │   ├── tesseract.rs   # Tesseract 实现
    │   │   └── mock.rs        # Mock 实现（测试用）
    │   └── tests/
    │       └── ocr_test.rs
    │
    ├── question_locator/      # 题号定位模块
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── types.rs       # 题号类型
    │   │   ├── patterns.rs    # 题号模式
    │   │   ├── matcher.rs     # 模式匹配
    │   │   ├── validator.rs   # 序列验证
    │   │   └── locator.rs     # 定位器
    │   └── tests/
    │       └── locator_test.rs
    │
    ├── question_segmentation/ # 题目分段模块
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── types.rs       # 分段类型
    │   │   ├── segmenter.rs   # 分段器
    │   │   ├── block_attribution.rs  # Block 归属
    │   │   ├── merger.rs             # 区域合并
    │   │   └── confidence.rs         # 置信度评估
    │   └── tests/
    │       └── segmentation_test.rs
    │
    ├── common/                # 通用模块
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs
    │   │   ├── geometry.rs    # 几何计算（Rect, Point）
    │   │   ├── error.rs       # 错误定义
    │   │   ├── logger.rs      # 日志
    │   │   └── debug.rs       # Debug 工具
    │   └── tests/
    │       └── geometry_test.rs
    │
    └── test_data/             # 测试数据
        ├── images/            # 测试图片
        │   ├── simple_01.jpg
        │   ├── multi_column.jpg
        │   └── with_noise.jpg
        └── golden/            # Golden Test 数据
            └── simple_01_expected.json
```

## 模块职责说明

### Flutter App

#### `lib/ffi/`
- **detector_ffi.dart**: 封装所有 FFI 调用，提供类型安全的 Dart API
- **detector_bindings.dart**: 使用 `ffigen` 自动生成的 C 绑定
- **native_library.dart**: 跨平台动态库加载逻辑

#### `lib/models/`
- **question_box.dart**: 题目框数据模型，包含位置、置信度等
- **detection_result.dart**: 完整检测结果，包含多个题目框
- **detector_config.dart**: 检测配置模型，对应 Rust 配置
- **debug_info.dart**: Debug 信息模型

#### `lib/services/`
- **detector_service.dart**: 业务逻辑层，管理检测器生命周期，处理异步调用

#### `lib/widgets/`
- **image_viewer.dart**: 支持缩放、平移的图片查看器
- **question_overlay.dart**: 在图片上绘制题目框的 CustomPainter
- **debug_panel.dart**: 显示 Debug 信息和中间结果

#### `lib/screens/`
- **home_screen.dart**: 主页，选择图片
- **detection_screen.dart**: 检测结果展示页

### Rust Core

#### `detector_ffi/`
FFI 边界层，负责：
- 导出 C ABI 函数
- 管理 Rust 对象的生命周期（Handle）
- 序列化/反序列化 JSON
- 捕获 panic，转换错误
- 线程安全保证

#### `detector_core/`
核心协调层，负责：
- 协调各模块执行
- 实现完整的检测 Pipeline
- 管理配置
- 生成最终结果

#### `image_processing/`
图像处理模块，包含：
- **grayscale**: RGB 转灰度
- **denoise**: 高斯模糊、中值滤波
- **binarization**: OTSU、自适应二值化
- **deskew**: Hough 直线检测、旋转校正
- **enhance**: 对比度、锐化

独立性强，可单独测试。

#### `block_detection/`
区块检测模块，包含：
- **connected_components**: 连通域标记（CCL）
- **projection**: 水平/垂直投影分析
- **morphology**: 膨胀、腐蚀、开闭运算
- **clustering**: DBSCAN / 层次聚类合并近邻 block
- **column_detector**: 分栏检测（垂直投影波谷）

#### `ocr_adapter/`
OCR 抽象层，包含：
- **traits**: `OcrEngine` trait 定义
- **tesseract**: Tesseract 适配器（基于 `tesseract-rs` 或 `leptess`）
- **mock**: 用于测试的 Mock 引擎

设计为可插拔，未来可添加 PaddleOCR、EasyOCR 等。

#### `question_locator/`
题号定位模块，包含：
- **patterns**: 题号正则模式库
- **matcher**: 对 OCR 结果或 block 应用模式匹配
- **validator**: 验证题号序列连续性、合理性
- **locator**: 主定位逻辑，结合几何位置与 OCR

#### `question_segmentation/`
题目分段模块，包含：
- **segmenter**: 主分段逻辑
- **block_attribution**: 判定每个 block 属于哪道题
- **merger**: 合并属于同一题的多个 block
- **confidence**: 对分段结果评分，支持多假设

#### `common/`
通用工具模块：
- **geometry**: `Rect`, `Point`, `Size` 等几何类型，包含交并比、包含关系等工具函数
- **error**: 统一错误类型定义
- **logger**: 日志初始化与配置
- **debug**: Debug 信息生成、图像标注工具

## 编译与集成

### Rust 编译

```bash
# 开发环境编译（当前平台）
cd rust_core
cargo build --release

# Android 交叉编译
cargo ndk -t arm64-v8a build --release

# iOS 编译
cargo build --target aarch64-apple-ios --release

# 生成 C 头文件
cd detector_ffi
cbindgen --config cbindgen.toml --crate detector_ffi --output detector.h
```

### Flutter 集成

```yaml
# pubspec.yaml
dependencies:
  ffi: ^2.0.0

dev_dependencies:
  ffigen: ^9.0.0
```

```bash
# 生成 FFI 绑定
flutter pub run ffigen --config ffigen.yaml

# 拷贝动态库到 Flutter 项目
# Android: flutter_app/android/src/main/jniLibs/
# iOS: flutter_app/ios/Frameworks/
# Windows: flutter_app/windows/
```

## 测试策略

### Rust 单元测试

每个模块都有 `tests/` 目录，包含：
- 算法正确性测试
- 边界条件测试
- 性能基准测试

```bash
cargo test
cargo test --release -- --nocapture
```

### Golden Test

使用测试图片和预期结果进行回归测试：

```rust
#[test]
fn test_simple_paper_detection() {
    let result = detect("test_data/images/simple_01.jpg");
    let expected = load_json("test_data/golden/simple_01_expected.json");
    assert_detection_match(result, expected, tolerance = 0.1);
}
```

### Flutter 集成测试

```dart
testWidgets('Detection screen shows question boxes', (tester) async {
  // ...
});
```

## 依赖管理

### Rust 主要依赖

```toml
# 图像处理
image = "0.24"          # 基础图像库
imageproc = "0.23"      # 图像处理算法

# OCR（可选）
tesseract = { version = "0.14", optional = true }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# FFI
libc = "0.2"

# 日志
log = "0.4"
env_logger = "0.10"

# 正则
regex = "1.10"

# 错误处理
anyhow = "1.0"
thiserror = "1.0"
```

### Flutter 主要依赖

```yaml
dependencies:
  flutter:
    sdk: flutter
  ffi: ^2.0.0
  path: ^1.8.0
  image_picker: ^1.0.0

dev_dependencies:
  ffigen: ^9.0.0
```

