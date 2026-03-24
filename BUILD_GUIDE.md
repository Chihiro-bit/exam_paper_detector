# 构建指南

本文档说明如何编译和运行项目。

## 环境要求

### 必需软件

1. **Flutter SDK** >= 3.0.0
   ```bash
   flutter --version
   ```

2. **Rust** >= 1.70.0
   ```bash
   rustc --version
   cargo --version
   ```

3. **flutter_rust_bridge_codegen**
   ```bash
   cargo install flutter_rust_bridge_codegen --version 2.0.0
   ```

### 可选工具

- **cbindgen** (用于生成 C 头文件)
  ```bash
  cargo install cbindgen
  ```

## 构建步骤

### 1. 克隆项目

```bash
cd exam_paper_detector
```

### 2. 生成 FFI 绑定代码

使用 flutter_rust_bridge_codegen 生成 Dart 和 Rust 的绑定代码：

```bash
flutter_rust_bridge_codegen generate
```

这会生成：
- `lib/src/bridge_generated.dart` - Dart 绑定
- `rust/src/bridge_generated.rs` - Rust 绑定
- `rust/src/bridge_generated.h` - C 头文件（可选）

### 3. 编译 Rust 核心库

#### Windows

```bash
cd rust
cargo build --release
```

编译产物位于 `rust/target/release/exam_paper_detector.dll`

#### Linux

```bash
cd rust
cargo build --release
```

编译产物位于 `rust/target/release/libexam_paper_detector.so`

#### macOS

```bash
cd rust
cargo build --release
```

编译产物位于 `rust/target/release/libexam_paper_detector.dylib`

#### Android (交叉编译)

首先安装 Android NDK 和目标架构：

```bash
# 安装 NDK
# 下载 Android NDK 并设置环境变量

# 添加 Android 目标
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android

# 使用 cargo-ndk 编译
cargo install cargo-ndk
cd rust
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build --release
```

#### iOS (交叉编译)

```bash
# 添加 iOS 目标
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim

# 编译
cd rust
cargo build --release --target aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
```

### 4. 拷贝动态库到 Flutter 项目

需要将编译好的动态库拷贝到对应平台目录：

#### Windows
```bash
copy rust\target\release\exam_paper_detector.dll windows\
```

#### Linux
```bash
cp rust/target/release/libexam_paper_detector.so linux/
```

#### macOS
```bash
cp rust/target/release/libexam_paper_detector.dylib macos/
```

#### Android
```bash
# 拷贝到 JNI 库目录
mkdir -p android/src/main/jniLibs/arm64-v8a
mkdir -p android/src/main/jniLibs/armeabi-v7a
mkdir -p android/src/main/jniLibs/x86_64

cp rust/target/aarch64-linux-android/release/libexam_paper_detector.so android/src/main/jniLibs/arm64-v8a/
cp rust/target/armv7-linux-androideabi/release/libexam_paper_detector.so android/src/main/jniLibs/armeabi-v7a/
cp rust/target/x86_64-linux-android/release/libexam_paper_detector.so android/src/main/jniLibs/x86_64/
```

#### iOS
```bash
# 拷贝到 Frameworks 目录
mkdir -p ios/Frameworks
cp rust/target/aarch64-apple-ios/release/libexam_paper_detector.a ios/Frameworks/
```

### 5. 运行示例应用

```bash
cd example
flutter pub get
flutter run
```

## 开发工作流

### 修改 Rust 代码后

1. 重新编译 Rust 库
   ```bash
   cd rust
   cargo build --release
   ```

2. 拷贝动态库（如上所述）

3. 重启 Flutter 应用

### 修改 API 接口后

1. 重新生成绑定代码
   ```bash
   flutter_rust_bridge_codegen generate
   ```

2. 重新编译 Rust 库

3. 重启 Flutter 应用

## 一键构建脚本

### Windows (build_all.bat)

创建 `build_all.bat`:

```batch
@echo off
echo Building Exam Paper Detector...

echo Step 1: Generating FFI bindings...
flutter_rust_bridge_codegen generate
if %errorlevel% neq 0 exit /b %errorlevel%

echo Step 2: Building Rust library...
cd rust
cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%
cd ..

echo Step 3: Copying library...
copy rust\target\release\exam_paper_detector.dll windows\
if %errorlevel% neq 0 exit /b %errorlevel%

echo Step 4: Getting Flutter dependencies...
cd example
flutter pub get
cd ..

echo Build complete!
echo Run 'cd example && flutter run' to start the app
```

### Linux/macOS (build_all.sh)

创建 `build_all.sh`:

```bash
#!/bin/bash
set -e

echo "Building Exam Paper Detector..."

echo "Step 1: Generating FFI bindings..."
flutter_rust_bridge_codegen generate

echo "Step 2: Building Rust library..."
cd rust
cargo build --release
cd ..

echo "Step 3: Copying library..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    cp rust/target/release/libexam_paper_detector.dylib macos/
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    cp rust/target/release/libexam_paper_detector.so linux/
fi

echo "Step 4: Getting Flutter dependencies..."
cd example
flutter pub get
cd ..

echo "Build complete!"
echo "Run 'cd example && flutter run' to start the app"
```

运行：
```bash
chmod +x build_all.sh
./build_all.sh
```

## 测试

### Rust 单元测试

```bash
cd rust
cargo test
```

### Rust 集成测试

```bash
cd rust
cargo test --test integration_test
```

### Flutter 测试

```bash
cd example
flutter test
```

## 故障排查

### 问题：flutter_rust_bridge_codegen 未找到

**解决方案**：
```bash
cargo install flutter_rust_bridge_codegen --version 2.0.0
```

### 问题：Rust 编译失败

**解决方案**：
1. 检查 Rust 版本：`rustc --version`
2. 更新 Rust：`rustup update`
3. 清理并重新编译：`cargo clean && cargo build --release`

### 问题：Flutter 找不到动态库

**解决方案**：
1. 确认动态库已编译：检查 `rust/target/release/` 目录
2. 确认库已拷贝到正确位置
3. 重启 Flutter 应用

### 问题：API 调用报错 UnimplementedError

**解决方案**：
1. 运行 `flutter_rust_bridge_codegen generate` 生成绑定代码
2. 重新编译 Rust 库
3. 确保库文件在正确位置

## 性能优化

### Release 模式编译

确保使用 `--release` 标志编译 Rust 代码，Debug 模式性能差距巨大。

### LTO (Link Time Optimization)

在 `rust/Cargo.toml` 中启用 LTO：

```toml
[profile.release]
lto = true
opt-level = 3
strip = true
```

### 并行编译

使用多核编译：

```bash
cargo build --release -j 8  # 使用 8 个并行任务
```

## 相关资源

- [flutter_rust_bridge 文档](https://cjycode.com/flutter_rust_bridge/)
- [Rust 交叉编译指南](https://rust-lang.github.io/rustup/cross-compilation.html)
- [Flutter Plugin 开发指南](https://docs.flutter.dev/development/packages-and-plugins/developing-packages)
