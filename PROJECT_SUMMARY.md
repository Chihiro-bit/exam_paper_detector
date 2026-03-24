# 项目总结文档

## 项目概述

本项目实现了一个基于 **Flutter + Rust + flutter_rust_bridge** 的跨平台试卷题目识别与框选系统。

### 核心特点

1. ✅ **完全跨平台**：支持 Android、iOS、Windows、macOS、Linux
2. ✅ **核心逻辑在 Rust**：不依赖原生平台 API
3. ✅ **类型安全**：使用 flutter_rust_bridge 自动生成绑定
4. ✅ **高性能**：Rust 实现的图像处理和算法
5. ✅ **易于扩展**：模块化设计，各组件独立可替换

## 技术栈

### Rust 端
- **image** / **imageproc**: 图像处理
- **serde** / **serde_json**: 序列化
- **regex**: 题号模式匹配
- **flutter_rust_bridge**: FFI 桥接

### Flutter 端
- **flutter_rust_bridge**: Rust 绑定
- **ffi**: 原生互操作
- **Material Design**: UI 组件

## 架构设计

### 模块划分

```
Rust Core (核心算法)
├── preprocessing      # 图像预处理
├── block_detection    # 区块检测
├── ocr               # OCR 适配层
├── question_locator  # 题号定位
├── segmentation      # 题目分段
└── detector          # 检测器协调

Flutter (UI + 业务)
├── models            # 数据模型
├── detector_service  # 服务层
└── widgets           # UI 组件
```

### 数据流

```
用户选择图片
    ↓
Flutter: DetectorService.processImage()
    ↓
FFI Bridge (flutter_rust_bridge)
    ↓
Rust: Detector.process_image()
    ├─ 1. 预处理 (grayscale, denoise, binarize)
    ├─ 2. Block 检测 (connected components)
    ├─ 3. OCR 识别 (可选)
    ├─ 4. 题号定位 (pattern matching + geometry)
    └─ 5. 题目分段 (block attribution + merging)
    ↓
返回 DetectionResult (JSON)
    ↓
Flutter 显示结果
```

## 核心算法

### 1. 图像预处理

**目的**：提升后续处理的准确性

**步骤**：
- 灰度化：RGB → Grayscale
- 去噪：高斯模糊
- 二值化：OTSU 或自适应阈值
- （可选）倾斜校正、对比度增强

**实现位置**：`rust/src/preprocessing.rs`

### 2. Block 检测

**目的**：检测所有文本候选区域

**方法**：
- **连通域分析**：标记前景像素的连通区域
- **投影分析**：水平/垂直投影找行列
- **聚类合并**：合并近邻 blocks

**关键点**：不依赖 OCR，纯几何分析

**实现位置**：`rust/src/block_detection.rs`

### 3. 题号定位

**目的**：找到每道题的题号位置

**策略**：
1. **基于 OCR**：
   - 对所有 blocks 进行 OCR
   - 用正则匹配题号模式（`1.`, `(1)`, `一、` 等）
   - 验证题号序列连续性

2. **基于几何**（OCR 失败时的回退）：
   - 查找左侧对齐的小 blocks
   - 检测等间距模式
   - 推断题号

**实现位置**：`rust/src/question_locator.rs`

### 4. 题目分段

**目的**：将 blocks 分组为完整题目

**方法**：
- 基于题号锚点，定义每题的垂直范围
- 将范围内的 blocks 归属到对应题目
- 合并所有 blocks 为题目边界框
- 计算置信度

**置信度评分**：
- 题号识别置信度 × 0.4
- 几何一致性 × 0.3
- Block 数量合理性 × 0.2
- 其他 × 0.1

**实现位置**：`rust/src/segmentation.rs`

## 关键设计决策

### 1. 为什么使用 flutter_rust_bridge？

**传统 FFI 方式的问题**：
- 需要手动编写 C ABI 绑定
- 类型不安全，容易出错
- Dart 和 Rust 数据结构需手动序列化
- 维护成本高

**flutter_rust_bridge 的优势**：
- ✅ 自动生成类型安全的绑定
- ✅ 支持复杂数据类型（struct, enum, Vec, Option）
- ✅ 自动处理内存管理
- ✅ 异步支持
- ✅ 更好的开发体验

### 2. 为什么核心逻辑在 Rust？

**优势**：
- ✅ 高性能：接近 C/C++ 的性能
- ✅ 内存安全：编译时保证无内存泄漏
- ✅ 跨平台：一次编译，多平台使用
- ✅ 丰富的生态：image、imageproc 等成熟库
- ✅ 易于维护：类型系统强大，重构安全

### 3. 为什么不依赖平台 OCR API？

**原因**：
- ❌ iOS Vision / Android ML Kit 不跨平台
- ❌ 不同平台 API 差异大，难以统一
- ❌ 受平台限制，无法离线使用

**我们的方案**：
- ✅ OCR 作为可选模块
- ✅ 几何分析为主，OCR 为辅
- ✅ 支持多种 OCR 引擎（Tesseract, PaddleOCR, 自定义）
- ✅ 可降级到无 OCR 模式

### 4. 为什么优先几何分析？

**试卷的特点**：
- 版面结构化：题号、题干、选项有固定模式
- 几何规律强：对齐、间距、缩进有规律
- 噪声多样：模糊、倾斜、阴影等影响 OCR

**几何分析的优势**：
- ✅ 鲁棒性高：对噪声容忍度好
- ✅ 速度快：不需要深度学习模型
- ✅ 可解释：规则清晰，易于调试
- ✅ 可回退：OCR 失败时仍能工作

## 已实现功能

### ✅ 核心功能

- [x] 图像预处理（灰度化、去噪、二值化）
- [x] 连通域检测
- [x] Block 聚类合并
- [x] 题号模式匹配
- [x] 题号定位（OCR + 几何）
- [x] 题目分段
- [x] 置信度评估
- [x] FFI 桥接（flutter_rust_bridge）
- [x] Flutter UI 示例

### ✅ 辅助功能

- [x] 配置管理（JSON 格式）
- [x] Debug 模式（中间结果保存）
- [x] 日志系统
- [x] 单元测试框架
- [x] 构建脚本

## 待实现功能

### 🚧 高优先级

- [ ] 倾斜校正（Hough 变换）
- [ ] 透视校正
- [ ] 分栏检测与处理
- [ ] 选项检测（A/B/C/D）
- [ ] Tesseract OCR 集成
- [ ] 更多测试用例
- [ ] 性能优化（并行处理）

### 📋 中优先级

- [ ] 图片区域检测
- [ ] 表格检测
- [ ] 手写体支持
- [ ] 多假设生成与选择
- [ ] Golden Test 数据集
- [ ] CI/CD 配置

### 💡 低优先级

- [ ] PaddleOCR 集成
- [ ] 深度学习模型支持
- [ ] 云端处理支持
- [ ] 批量处理优化
- [ ] 可视化 Debug 工具

## 性能指标

### 目标（单页 A4 试卷）

| 指标 | 目标值 | 当前状态 |
|------|--------|----------|
| 处理时间 | < 3秒 | 待测试 |
| 题号识别率 | > 90% | 待测试 |
| 题目分割准确率 | > 85% | 待测试 |
| 内存占用 | < 100MB | 待测试 |
| 包体积增加 | < 10MB | 待测试 |

### 性能优化方向

1. **并行处理**：使用 rayon 并行处理多个 blocks
2. **图像金字塔**：多分辨率处理
3. **缓存优化**：缓存中间结果
4. **算法优化**：优化连通域标记算法
5. **SIMD**：使用 SIMD 加速图像处理

## 测试策略

### 单元测试

每个模块都有单元测试：
```bash
cd rust
cargo test
```

### 集成测试

使用真实图片测试完整流程：
```bash
cargo test --test integration_test
```

### Golden Test

建立测试数据集，验证输出一致性：
- 输入：标准试卷图片
- 期望输出：题目框坐标 JSON
- 验证：比较实际输出与期望输出

### 性能测试

使用 criterion 进行基准测试：
```bash
cargo bench
```

## 开发规范

### Rust 代码规范

1. **命名**：
   - 模块：snake_case
   - 结构体/枚举：PascalCase
   - 函数/变量：snake_case
   - 常量：UPPER_SNAKE_CASE

2. **注释**：
   - 模块级别：`//!` 文档注释
   - 公共 API：`///` 文档注释
   - 内部逻辑：`//` 普通注释

3. **错误处理**：
   - 使用 `Result<T, anyhow::Error>`
   - 不使用 `unwrap()`，改用 `?` 或 `expect()`
   - FFI 边界捕获所有错误

4. **测试**：
   - 每个模块都有 `#[cfg(test)]` 测试
   - 公共 API 必须有测试覆盖

### Flutter 代码规范

1. **命名**：
   - 文件：snake_case
   - 类：PascalCase
   - 变量/方法：camelCase
   - 私有成员：_leadingUnderscore

2. **异步**：
   - 使用 `async/await`
   - 错误使用 `try-catch` 处理

3. **状态管理**：
   - 简单场景：StatefulWidget
   - 复杂场景：（待定，可选 Provider / Riverpod）

## 故障排查

### 常见问题

**1. 题号识别失败**

可能原因：
- 题号格式不在预定义模式中
- OCR 识别错误
- 图片质量太差

解决方案：
- 添加自定义题号模式
- 调整 OCR 参数
- 启用 Debug 模式查看中间结果

**2. 题目分段不准确**

可能原因：
- Block 检测遗漏或过多
- 题号定位错误
- 垂直间距判断失误

解决方案：
- 调整二值化参数
- 检查题号定位结果
- 调整分段阈值

**3. 性能慢**

可能原因：
- Debug 模式编译
- 图片分辨率过高
- 没有启用优化

解决方案：
- 使用 Release 模式：`cargo build --release`
- 缩小图片尺寸
- 启用 LTO：在 Cargo.toml 中配置

## 部署指南

### Android APK

```bash
cd example
flutter build apk --release
```

### iOS IPA

```bash
cd example
flutter build ios --release
```

### Windows EXE

```bash
cd example
flutter build windows --release
```

### macOS APP

```bash
cd example
flutter build macos --release
```

### Linux Binary

```bash
cd example
flutter build linux --release
```

## 贡献指南

### 提交代码前

1. 运行测试：`cargo test && flutter test`
2. 格式化代码：`cargo fmt && dart format .`
3. Lint 检查：`cargo clippy && flutter analyze`
4. 更新文档：如有 API 变更，更新对应文档

### Git 工作流

1. 创建功能分支：`git checkout -b feature/xxx`
2. 提交代码：`git commit -m "feat: add xxx"`
3. 推送分支：`git push origin feature/xxx`
4. 创建 Pull Request

### Commit Message 规范

```
<type>(<scope>): <subject>

<body>

<footer>
```

类型：
- feat: 新功能
- fix: 修复
- docs: 文档
- style: 格式
- refactor: 重构
- test: 测试
- chore: 构建/工具

## 许可证

MIT License

## 联系方式

- Issue: GitHub Issues
- Email: your@email.com

---

**项目状态**：🚧 开发中

**最后更新**：2024-01-XX
