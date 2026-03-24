# 试卷题目识别与框选系统

[English](README.md) | 简体中文

一个基于 **Flutter + Rust + flutter_rust_bridge** 的跨平台试卷题目自动识别与框选系统。

## 特性

- ✅ **完全跨平台**：支持 Android、iOS、Windows、macOS、Linux
- ✅ **高性能**：Rust 实现核心算法，接近原生性能
- ✅ **类型安全**：flutter_rust_bridge 自动生成类型安全的绑定
- ✅ **无需原生 API**：不依赖平台特定的 Vision/ML Kit
- ✅ **离线可用**：所有处理在本地完成
- ✅ **易于扩展**：模块化设计，算法可替换

## 快速开始

### 环境要求

- Flutter SDK >= 3.0.0
- Rust >= 1.70.0
- flutter_rust_bridge_codegen

### 安装

```bash
# 1. 安装 flutter_rust_bridge_codegen
cargo install flutter_rust_bridge_codegen --version 2.0.0

# 2. 克隆项目
git clone <repository-url>
cd exam_paper_detector

# 3. 一键构建（Windows）
build_all.bat

# 或手动构建
flutter_rust_bridge_codegen generate
cd rust && cargo build --release && cd ..
flutter pub get
```

### 运行示例

```bash
cd example
flutter run
```

## 使用方法

### 基本用法

```dart
import 'package:exam_paper_detector/exam_paper_detector.dart';

// 1. 创建检测器服务
final detector = DetectorService();

// 2. 初始化
await detector.initialize();

// 3. 处理图片
final result = await detector.processImage(
  '/path/to/exam_paper.jpg',
  includeDebug: true,
);

// 4. 获取结果
if (result.success) {
  print('找到 ${result.questionCount} 道题');
  for (var question in result.questions) {
    print('题目 ${question.questionId}: (${question.x}, ${question.y})');
  }
}

// 5. 释放资源
await detector.dispose();
```

### 自定义配置

```dart
// 创建自定义配置
final config = DetectorConfig(
  preprocessing: PreprocessingConfig(
    enableDeskew: true,
    enableDenoise: true,
    binarizationMethod: 'Adaptive',
    contrastEnhancement: 1.2,
  ),
  questionPatterns: QuestionPattern.defaultPatterns(),
  debug: DebugConfig(
    saveIntermediate: true,
    outputDir: '/tmp/debug',
    verbose: true,
  ),
);

// 使用配置初始化
await detector.initialize(config);
```

## 核心算法

### 1. 图像预处理
- 灰度化、去噪、二值化
- 倾斜校正、对比度增强

### 2. 区块检测
- 连通域分析
- 投影分析
- 区块聚类合并

### 3. 题号定位
- 正则模式匹配（支持 `1.`, `(1)`, `一、` 等）
- 几何位置约束
- 序列连续性验证

### 4. 题目分段
- 基于题号锚点的区域划分
- Block 归属判定
- 置信度评估

详细架构设计请参考 [ARCHITECTURE.md](ARCHITECTURE.md)

## 项目结构

```
exam_paper_detector/
├── rust/                   # Rust 核心库
│   ├── src/
│   │   ├── api.rs         # Flutter 可调用的 API
│   │   ├── detector.rs    # 检测器核心
│   │   ├── preprocessing.rs
│   │   ├── block_detection.rs
│   │   ├── ocr.rs
│   │   ├── question_locator.rs
│   │   └── segmentation.rs
│   └── Cargo.toml
│
├── lib/                    # Flutter/Dart 代码
│   ├── src/
│   │   ├── models/        # 数据模型
│   │   ├── detector_service.dart
│   │   ├── bridge_generated.dart  # 自动生成
│   │   └── bridge_definitions.dart
│   └── exam_paper_detector.dart
│
└── example/                # 示例应用
    └── lib/main.dart
```

## 性能

| 指标 | 目标值 |
|------|--------|
| 处理时间 | < 3秒/页 |
| 题号识别率 | > 90% |
| 题目分割准确率 | > 85% |

## 开发指南

### 修改 Rust 代码

1. 编辑 `rust/src/*.rs`
2. 重新编译：`cd rust && cargo build --release`
3. 拷贝库：`copy rust\target\release\exam_paper_detector.dll windows\`
4. 重启 Flutter 应用

### 修改 API 接口

1. 编辑 `rust/src/api.rs`
2. 重新生成绑定：`flutter_rust_bridge_codegen generate`
3. 重新编译 Rust 库
4. 更新 Dart 代码

### 运行测试

```bash
# Rust 单元测试
cd rust
cargo test

# Flutter 测试
cd example
flutter test
```

## 文档

- [架构设计](ARCHITECTURE.md) - 详细的架构设计文档
- [目录结构](DIRECTORY_STRUCTURE.md) - 项目目录说明
- [MVP 路线图](MVP_ROADMAP.md) - 开发计划
- [构建指南](BUILD_GUIDE.md) - 详细的构建步骤
- [项目总结](PROJECT_SUMMARY.md) - 完整的项目总结

## 技术栈

### Rust
- image / imageproc - 图像处理
- serde / serde_json - 序列化
- regex - 正则表达式
- flutter_rust_bridge - FFI 桥接

### Flutter
- flutter_rust_bridge - Rust 绑定
- ffi - 原生互操作

## 路线图

### 已完成 ✅
- [x] 基础架构设计
- [x] 图像预处理模块
- [x] Block 检测模块
- [x] 题号定位模块
- [x] 题目分段模块
- [x] FFI 桥接（flutter_rust_bridge）
- [x] Flutter 示例应用

### 进行中 🚧
- [ ] 倾斜校正
- [ ] 分栏检测
- [ ] Tesseract OCR 集成
- [ ] 性能优化

### 计划中 📋
- [ ] 手写体支持
- [ ] 深度学习模型
- [ ] CI/CD 配置

## 常见问题

**Q: 为什么使用 Rust 而不是 Dart？**

A: Rust 提供了接近 C/C++ 的性能，同时具有内存安全保证。图像处理是计算密集型任务，Rust 是最佳选择。

**Q: 是否支持手写题号？**

A: 目前主要支持印刷体。手写体需要集成 OCR 引擎（如 Tesseract）或深度学习模型。

**Q: 能否在没有 OCR 的情况下工作？**

A: 可以！我们优先使用几何分析，OCR 只是辅助。即使 OCR 完全失败，系统仍能基于几何位置推断题号。

**Q: 如何提高识别准确率？**

A:
1. 提供高质量图片（清晰、正面、光照均匀）
2. 添加自定义题号模式
3. 调整二值化参数
4. 启用 Debug 模式查看中间结果

## 贡献

欢迎提交 Issue 和 Pull Request！

### 提交代码前
1. 运行测试：`cargo test && flutter test`
2. 格式化代码：`cargo fmt && dart format .`
3. Lint 检查：`cargo clippy && flutter analyze`

## 许可证

MIT License

## 致谢

- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) - 出色的 Flutter-Rust 桥接工具
- [image-rs](https://github.com/image-rs/image) - Rust 图像处理库
- [imageproc](https://github.com/image-rs/imageproc) - 图像处理算法

---

**Status**: 🚧 开发中

如有问题，请提交 Issue 或联系维护者。
