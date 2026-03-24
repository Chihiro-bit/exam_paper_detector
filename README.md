# 试卷题目识别与框选系统

基于 Flutter + Rust + flutter_rust_bridge 的跨平台试卷题目自动识别系统。

# 使用示例图

![使用示例](screenshot/img.png)

## 架构特点

- **Flutter Plugin**: 标准 Flutter 插件架构
- **flutter_rust_bridge**: 自动生成类型安全的 Rust-Dart 绑定
- **核心算法全在 Rust**: 图像处理、版面分析、题目分割
- **跨平台**: Android、iOS、Windows、macOS、Linux

## 项目结构

```
exam_paper_detector/
├── lib/                    # Dart 代码
├── rust/                   # Rust 核心代码
├── example/                # 示例 App
├── android/                # Android 平台
├── ios/                    # iOS 平台
├── windows/                # Windows 平台
├── macos/                  # macOS 平台
└── linux/                  # Linux 平台
```

## 快速开始

### 环境要求

- Flutter SDK >= 3.0.0
- Rust >= 1.70.0
- flutter_rust_bridge_codegen

### 安装 flutter_rust_bridge_codegen

```bash
cargo install flutter_rust_bridge_codegen
```

### 构建

```bash
# 生成绑定代码
flutter_rust_bridge_codegen generate

# 运行示例
cd example
flutter run
```

## 开发流程

1. 在 `rust/src/api.rs` 中定义 Rust API
2. 运行 `flutter_rust_bridge_codegen generate` 生成绑定
3. 在 Dart 中调用生成的 API

## 功能特性

- ✅ 图像预处理（去噪、二值化、校正）
- ✅ 智能版面分析
- ✅ 题号自动定位
- ✅ 题目区域分割
- ✅ 置信度评估
- ✅ Debug 可视化

## License

MIT
