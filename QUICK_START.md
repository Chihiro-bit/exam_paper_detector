# 快速入门指南

本指南帮助你在 5 分钟内运行起来这个项目。

## 前置条件

确保你已经安装：

- ✅ Flutter SDK (>= 3.0.0)
- ✅ Rust (>= 1.70.0)

检查版本：
```bash
flutter --version
rustc --version
```

## 步骤 1：安装 flutter_rust_bridge_codegen

```bash
cargo install flutter_rust_bridge_codegen --version 2.0.0
```

安装大约需要 5-10 分钟。

## 步骤 2：克隆并进入项目

```bash
cd D:\project\demo\exam_paper_detector
```

## 步骤 3：一键构建

### Windows

```bash
build_all.bat
```

### Linux/macOS

```bash
chmod +x build_all.sh
./build_all.sh
```

这个脚本会自动完成：
1. ✅ 生成 FFI 绑定代码
2. ✅ 编译 Rust 库
3. ✅ 拷贝动态库到正确位置
4. ✅ 安装 Flutter 依赖

## 步骤 4：运行示例应用

```bash
cd example
flutter run -d windows  # Windows
# 或
flutter run -d macos    # macOS
# 或
flutter run -d linux    # Linux
```

## 步骤 5：测试功能

在示例应用中：
1. 点击"初始化检测器"按钮
2. 选择一张试卷图片
3. 点击"处理图片"
4. 查看检测结果

## 手动测试（命令行）

### 测试 Rust 库

```bash
cd rust
cargo test
```

预期输出：
```
running 10 tests
test geometry::tests::test_rect_iou ... ok
test geometry::tests::test_rect_union ... ok
...
test result: ok. 10 passed; 0 failed
```

### 测试 Dart API

```bash
cd example
flutter test
```

## 验证安装

运行以下命令验证一切正常：

```bash
# 1. 检查 Rust 库是否编译成功
dir rust\target\release\exam_paper_detector.dll  # Windows
# 或
ls rust/target/release/libexam_paper_detector.so  # Linux
# 或
ls rust/target/release/libexam_paper_detector.dylib  # macOS

# 2. 检查 Flutter 依赖
flutter pub get
flutter doctor

# 3. 检查生成的绑定代码
dir lib\src\bridge_generated.dart  # Windows
# 或
ls lib/src/bridge_generated.dart  # Linux/macOS
```

## 常见问题

### 问题 1：`flutter_rust_bridge_codegen` 命令未找到

**原因**：没有安装或未添加到 PATH

**解决方案**：
```bash
cargo install flutter_rust_bridge_codegen --version 2.0.0

# 确认安装成功
flutter_rust_bridge_codegen --version
```

### 问题 2：Rust 编译失败

**原因**：Rust 版本过低或依赖问题

**解决方案**：
```bash
# 更新 Rust
rustup update

# 清理并重新编译
cd rust
cargo clean
cargo build --release
```

### 问题 3：Flutter 找不到动态库

**原因**：库文件未拷贝到正确位置

**解决方案**：
```bash
# Windows
copy rust\target\release\exam_paper_detector.dll windows\

# Linux
cp rust/target/release/libexam_paper_detector.so linux/

# macOS
cp rust/target/release/libexam_paper_detector.dylib macos/
```

### 问题 4：运行示例报错 `UnimplementedError`

**原因**：未生成 FFI 绑定代码

**解决方案**：
```bash
flutter_rust_bridge_codegen generate
```

## 下一步

现在你已经成功运行了项目！接下来可以：

1. **查看代码**：
   - Rust 核心：`rust/src/detector.rs`
   - Flutter 服务：`lib/src/detector_service.dart`
   - 示例应用：`example/lib/main.dart`

2. **阅读文档**：
   - [架构设计](ARCHITECTURE.md)
   - [完整构建指南](BUILD_GUIDE.md)
   - [项目总结](PROJECT_SUMMARY.md)

3. **尝试修改**：
   - 添加新的题号模式
   - 调整图像处理参数
   - 自定义 UI

4. **测试真实图片**：
   - 准备一些试卷图片
   - 放到 `test_data/images/` 目录
   - 使用示例应用测试

## 代码示例

### 最简单的使用

```dart
import 'package:exam_paper_detector/exam_paper_detector.dart';

void main() async {
  // 创建检测器
  final detector = DetectorService();

  // 初始化
  await detector.initialize();

  // 处理图片
  final result = await detector.processImage('/path/to/image.jpg');

  // 打印结果
  print('找到 ${result.questionCount} 道题');

  // 释放资源
  await detector.dispose();
}
```

### 带配置的使用

```dart
import 'package:exam_paper_detector/exam_paper_detector.dart';

void main() async {
  final detector = DetectorService();

  // 自定义配置
  final config = DetectorConfig(
    preprocessing: PreprocessingConfig(
      enableDeskew: true,
      enableDenoise: true,
      binarizationMethod: 'Adaptive',
      contrastEnhancement: 1.2,
    ),
    questionPatterns: [
      QuestionPattern(
        pattern: r'^\d+\.',
        patternType: 'Numbered',
        priority: 10,
      ),
    ],
    debug: DebugConfig(
      saveIntermediate: true,
      outputDir: '/tmp/debug',
      verbose: true,
    ),
  );

  await detector.initialize(config);

  final result = await detector.processImage(
    '/path/to/image.jpg',
    includeDebug: true,
  );

  // 遍历每道题
  for (var question in result.questions) {
    print('题目 ${question.questionId}:');
    print('  位置: (${question.x}, ${question.y})');
    print('  大小: ${question.width} x ${question.height}');
    print('  置信度: ${question.confidence}');
  }

  await detector.dispose();
}
```

## 性能提示

1. **使用 Release 模式编译 Rust**：
   ```bash
   cargo build --release
   ```
   Debug 模式会慢 10-100 倍！

2. **缩小图片尺寸**：
   如果图片过大（> 4000 像素），先缩小再处理。

3. **启用并行处理**（未来功能）：
   批量处理多张图片时会更快。

## 获取帮助

- 📖 阅读文档：[ARCHITECTURE.md](ARCHITECTURE.md)
- 🐛 提交 Issue：GitHub Issues
- 💬 讨论：GitHub Discussions

---

**恭喜！你已经成功运行了项目！** 🎉

现在可以开始探索代码，尝试自己的修改，或者用真实的试卷图片测试。
