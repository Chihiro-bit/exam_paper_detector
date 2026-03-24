@echo off
setlocal enabledelayedexpansion

echo ========================================
echo Building Exam Paper Detector
echo ========================================
echo.

:: 检查必需工具
echo Checking required tools...

where flutter >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Flutter not found. Please install Flutter SDK.
    exit /b 1
)

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Cargo not found. Please install Rust.
    exit /b 1
)

where flutter_rust_bridge_codegen >nul 2>nul
if %errorlevel% neq 0 (
    echo [WARNING] flutter_rust_bridge_codegen not found.
    echo Installing flutter_rust_bridge_codegen...
    cargo install flutter_rust_bridge_codegen --version 2.0.0
    if !errorlevel! neq 0 (
        echo [ERROR] Failed to install flutter_rust_bridge_codegen
        exit /b 1
    )
)

echo All required tools are available.
echo.

:: Step 1: 生成 FFI 绑定
echo ========================================
echo Step 1/5: Generating FFI bindings...
echo ========================================
flutter_rust_bridge_codegen generate
if %errorlevel% neq 0 (
    echo [ERROR] Failed to generate FFI bindings
    exit /b 1
)
echo [OK] FFI bindings generated
echo.

:: Step 2: 编译 Rust 库
echo ========================================
echo Step 2/5: Building Rust library...
echo ========================================
cd rust
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Failed to build Rust library
    cd ..
    exit /b 1
)
cd ..
echo [OK] Rust library built
echo.

:: Step 3: 创建目标目录
echo ========================================
echo Step 3/5: Preparing directories...
echo ========================================
if not exist windows mkdir windows
echo [OK] Directories prepared
echo.

:: Step 4: 拷贝动态库
echo ========================================
echo Step 4/5: Copying library files...
echo ========================================
copy /Y rust\target\release\exam_paper_detector.dll windows\
if %errorlevel% neq 0 (
    echo [ERROR] Failed to copy library
    exit /b 1
)
echo [OK] Library copied
echo.

:: Step 5: 安装 Flutter 依赖
echo ========================================
echo Step 5/5: Installing Flutter dependencies...
echo ========================================
flutter pub get
if %errorlevel% neq 0 (
    echo [ERROR] Failed to get Flutter dependencies
    exit /b 1
)

cd example
flutter pub get
if %errorlevel% neq 0 (
    echo [ERROR] Failed to get example dependencies
    cd ..
    exit /b 1
)
cd ..
echo [OK] Dependencies installed
echo.

:: 完成
echo ========================================
echo Build Complete!
echo ========================================
echo.
echo Next steps:
echo   1. Run tests:    cd rust ^&^& cargo test
echo   2. Run example:  cd example ^&^& flutter run
echo   3. Build release: cd example ^&^& flutter build windows --release
echo.

endlocal
