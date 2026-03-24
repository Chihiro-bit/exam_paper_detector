# MVP 开发路线图

## 总体策略

采用**自底向上、逐层验证**的开发方式：
1. 先建立基础设施（类型、错误、几何）
2. 再实现核心算法模块
3. 最后整合 FFI 和 Flutter UI
4. 每个阶段都有可验证的成果

## 第一阶段：基础设施与图像处理（Week 1-2）

### 目标
建立项目骨架，实现基础图像处理能力

### 任务清单

#### 1.1 项目初始化
- [x] 创建目录结构
- [ ] 配置 Cargo workspace
- [ ] 创建 Flutter 项目
- [ ] 配置 ffigen
- [ ] 编写构建脚本

#### 1.2 通用模块 (`common/`)
- [ ] 定义几何类型：`Rect`, `Point`, `Size`
- [ ] 实现几何运算：交并比、包含、距离
- [ ] 定义统一错误类型
- [ ] 配置日志系统
- [ ] 编写单元测试

**可验证成果**：
```rust
#[test]
fn test_rect_iou() {
    let r1 = Rect::new(0, 0, 100, 100);
    let r2 = Rect::new(50, 50, 100, 100);
    assert!((r1.iou(&r2) - 0.14).abs() < 0.01);
}
```

#### 1.3 图像处理模块 (`image_processing/`)
- [ ] 定义 `Image` 类型（封装 `image` crate）
- [ ] 实现灰度化
- [ ] 实现高斯去噪
- [ ] 实现 OTSU 二值化
- [ ] 实现自适应二值化
- [ ] 实现对比度增强
- [ ] 编写单元测试

**可验证成果**：
```rust
#[test]
fn test_binarization() {
    let img = Image::load("test_data/images/sample.jpg").unwrap();
    let binary = binarize(&img, BinarizationMethod::Otsu).unwrap();
    binary.save("output/binary.png").unwrap();
    // 人工验证二值化效果
}
```

**里程碑**：能够加载图片、预处理并保存结果

---

## 第二阶段：Block 检测（Week 3-4）

### 目标
实现版面分析，检测文本候选区域

### 任务清单

#### 2.1 连通域分析
- [ ] 实现 CCL（Connected Component Labeling）
- [ ] 过滤小噪点（面积阈值）
- [ ] 提取 bounding box
- [ ] 可视化标注

**可验证成果**：
```rust
#[test]
fn test_connected_components() {
    let binary = Image::load("test_data/binary.png").unwrap();
    let components = find_connected_components(&binary);
    assert!(components.len() > 0);
    // 保存标注图像
    save_annotated(&binary, &components, "output/components.png");
}
```

#### 2.2 投影分析
- [ ] 实现水平投影
- [ ] 实现垂直投影
- [ ] 检测空白分隔
- [ ] 检测行
- [ ] 检测列

**可验证成果**：
```rust
#[test]
fn test_projection() {
    let binary = Image::load("test_data/binary.png").unwrap();
    let lines = detect_lines_by_projection(&binary);
    assert!(lines.len() > 0);
    // 可视化投影直方图
}
```

#### 2.3 Block 聚类合并
- [ ] 计算 block 之间距离
- [ ] 实现层次聚类
- [ ] 合并近邻 block
- [ ] 按对齐关系分组

**可验证成果**：
```rust
#[test]
fn test_block_clustering() {
    let blocks = vec![/* ... */];
    let merged = cluster_blocks(&blocks, threshold);
    assert!(merged.len() < blocks.len());
}
```

#### 2.4 分栏检测
- [ ] 垂直投影检测分栏
- [ ] 识别单栏/双栏/多栏
- [ ] 划分栏边界

**里程碑**：输入试卷图片，输出所有文本 block 的位置框

---

## 第三阶段：OCR 与题号定位（Week 5-6）

### 目标
集成 OCR，识别题号位置

### 任务清单

#### 3.1 OCR 适配层
- [ ] 定义 `OcrEngine` trait
- [ ] 定义 OCR 类型：`TextBlock`, `OcrResult`
- [ ] 实现 Mock OCR（用于测试）
- [ ] （可选）实现 Tesseract 适配器

**可验证成果**：
```rust
#[test]
fn test_mock_ocr() {
    let ocr = MockOcrEngine::new();
    let img = Image::load("test.jpg").unwrap();
    let result = ocr.recognize(&img, &config).unwrap();
    assert!(result.len() > 0);
}
```

#### 3.2 题号模式库
- [ ] 定义题号正则模式
- [ ] 支持配置文件加载模式
- [ ] 实现模式匹配器

**可验证成果**：
```rust
#[test]
fn test_question_pattern() {
    let patterns = QuestionPatterns::default();
    assert!(patterns.is_match("1."));
    assert!(patterns.is_match("(1)"));
    assert!(patterns.is_match("一、"));
    assert!(!patterns.is_match("ABC"));
}
```

#### 3.3 题号定位器
- [ ] 对 OCR 结果应用模式匹配
- [ ] 结合位置约束（左侧、顶部）
- [ ] 序列连续性验证
- [ ] 置信度评分

**可验证成果**：
```rust
#[test]
fn test_question_locator() {
    let blocks = load_test_blocks();
    let ocr_results = load_test_ocr();
    let locator = QuestionLocator::new(patterns);
    let anchors = locator.locate(&blocks, &ocr_results).unwrap();
    assert_eq!(anchors.len(), 5); // 预期 5 道题
    assert_eq!(anchors[0].question_id, "1");
}
```

**里程碑**：能够在试卷图片上标注出题号位置

---

## 第四阶段：题目分段（Week 7-8）

### 目标
基于题号，将 blocks 分组为完整题目

### 任务清单

#### 4.1 Block 归属判定
- [ ] 实现垂直范围归属
- [ ] 实现缩进归属
- [ ] 实现选项检测（A/B/C/D）
- [ ] 实现图片区域检测

**可验证成果**：
```rust
#[test]
fn test_block_attribution() {
    let anchor = QuestionAnchor { y: 100, ... };
    let next_anchor = QuestionAnchor { y: 300, ... };
    let block = Block { y: 150, ... };
    assert!(belongs_to_question(&block, &anchor, &next_anchor));
}
```

#### 4.2 区域合并
- [ ] 合并属于同一题的 blocks
- [ ] 计算最小外接矩形
- [ ] 处理多栏情况

#### 4.3 置信度评估
- [ ] 题号识别置信度
- [ ] 几何一致性评分
- [ ] Block 归属合理性评分
- [ ] 综合评分

**可验证成果**：
```rust
#[test]
fn test_segmentation() {
    let image = load_test_image();
    let result = segment_questions(&image, &config).unwrap();
    assert_eq!(result.questions.len(), 10);
    for q in &result.questions {
        assert!(q.confidence > 0.5);
    }
}
```

**里程碑**：输入试卷图片，输出每道题的完整框选区域

---

## 第五阶段：核心整合与 FFI（Week 9-10）

### 目标
整合所有模块，暴露 FFI 接口

### 任务清单

#### 5.1 核心检测器
- [ ] 实现 `Detector` 结构
- [ ] 实现完整 Pipeline
- [ ] 配置管理
- [ ] Debug 信息生成

**可验证成果**：
```rust
#[test]
fn test_detector_pipeline() {
    let detector = Detector::new(config).unwrap();
    let result = detector.process_image("test.jpg").unwrap();
    assert!(result.questions.len() > 0);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
```

#### 5.2 FFI 层
- [ ] 实现 Handle 管理
- [ ] 实现 C API 函数
- [ ] JSON 序列化/反序列化
- [ ] 错误捕获与传递
- [ ] 内存安全保证

**可验证成果**：
```c
// C 测试程序
DetectorHandle* handle = detector_create("{...}");
char* result = detector_process_image(handle, "test.jpg", "{}");
printf("%s\n", result);
free_string(result);
detector_destroy(handle);
```

#### 5.3 生成 C 头文件
- [ ] 配置 cbindgen
- [ ] 生成 `detector.h`
- [ ] 编写使用文档

**里程碑**：Rust 库可以被 C 程序调用

---

## 第六阶段：Flutter 集成（Week 11-12）

### 目标
实现 Flutter UI，展示检测结果

### 任务清单

#### 6.1 FFI 绑定
- [ ] 配置 ffigen
- [ ] 生成 Dart 绑定
- [ ] 实现动态库加载
- [ ] 封装类型安全 API

**可验证成果**：
```dart
void main() {
  final detector = DetectorService();
  detector.initialize(config);
  final result = detector.processImage('test.jpg');
  print('Found ${result.questions.length} questions');
  detector.dispose();
}
```

#### 6.2 数据模型
- [ ] 实现 `QuestionBox` 模型
- [ ] 实现 `DetectionResult` 模型
- [ ] JSON 解析

#### 6.3 UI 组件
- [ ] 实现图片查看器（支持缩放、平移）
- [ ] 实现题目框 Overlay
- [ ] 实现 Debug 面板

**可验证成果**：
- 能够显示图片
- 能够在图片上绘制框选
- 能够缩放平移

#### 6.4 交互功能
- [ ] 选择图片
- [ ] 触发检测
- [ ] 显示结果
- [ ] 点击选题
- [ ] 手动修框（可选）

**里程碑**：完整的 Flutter App，能够检测并展示题目框

---

## 第七阶段：优化与完善（Week 13-14）

### 目标
提升鲁棒性，添加高级特性

### 任务清单

#### 7.1 算法优化
- [ ] 实现倾斜校正（Hough 变换）
- [ ] 实现透视校正
- [ ] 优化 Block 聚类算法
- [ ] 多假设生成与选择
- [ ] 回退机制

#### 7.2 性能优化
- [ ] 并行处理（rayon）
- [ ] 图像金字塔（多分辨率）
- [ ] 缓存中间结果

#### 7.3 测试完善
- [ ] 添加更多单元测试
- [ ] 建立 Golden Test 数据集
- [ ] 性能基准测试
- [ ] 集成测试

#### 7.4 文档与示例
- [ ] API 文档
- [ ] 使用教程
- [ ] 示例代码
- [ ] 常见问题

**里程碑**：生产级别的可用系统

---

## 每个阶段的验证标准

### 阶段 1：基础设施
- ✅ 能够加载、处理、保存图像
- ✅ 所有几何运算测试通过
- ✅ 错误处理机制完善

### 阶段 2：Block 检测
- ✅ 能够检测出所有文本区域
- ✅ Block 数量合理（不过多、不遗漏）
- ✅ 可视化结果清晰

### 阶段 3：题号定位
- ✅ 能够识别 90% 以上的题号
- ✅ 题号序列连续性正确
- ✅ 误报率 < 5%

### 阶段 4：题目分段
- ✅ 能够正确分割 85% 以上的题目
- ✅ 题目边界基本准确
- ✅ 选项、图片正确归属

### 阶段 5：FFI
- ✅ C 程序能够调用 Rust 库
- ✅ 无内存泄漏
- ✅ 错误能够正确传递

### 阶段 6：Flutter
- ✅ UI 流畅，无卡顿
- ✅ 框选准确显示
- ✅ 交互符合直觉

### 阶段 7：优化
- ✅ 处理时间 < 3 秒（单页）
- ✅ 对模糊、倾斜、噪点有容忍度
- ✅ 测试覆盖率 > 80%

---

## 当前进度

- [x] 架构设计完成
- [x] 目录结构设计完成
- [x] MVP 路线图制定完成
- [ ] 开始第一阶段开发

## 下一步行动

1. 创建 Rust workspace 和各个 crate
2. 实现 `common` 模块的几何类型
3. 实现 `image_processing` 模块的基础功能
4. 编写第一个可运行的测试

---

## 风险与挑战

### 技术风险

1. **OCR 识别率**
   - 缓解：不完全依赖 OCR，几何分析为主
   - 回退：提供手动模式

2. **复杂版式**
   - 缓解：MVP 先支持简单版式
   - 扩展：逐步添加规则

3. **性能问题**
   - 缓解：优化算法、并行处理
   - 降级：提供低质量快速模式

### 工程风险

1. **FFI 复杂性**
   - 缓解：充分测试内存安全
   - 工具：使用 Valgrind/ASAN 检查

2. **跨平台兼容**
   - 缓解：早期在多平台测试
   - CI：自动化多平台构建

3. **依赖管理**
   - 缓解：固定版本、最小依赖
   - 备选：关键算法自实现

---

## 成功指标

### MVP 成功标准（阶段 6 结束）

- ✅ 能够处理印刷体、单栏试卷
- ✅ 题号格式：`1.`, `(1)`, `一、`
- ✅ 识别率 > 80%
- ✅ 处理时间 < 5 秒/页
- ✅ Flutter App 可运行
- ✅ 至少支持一个平台（Windows/macOS/Linux）

### 完整版成功标准（阶段 7 结束）

- ✅ 支持双栏排版
- ✅ 支持手写体（OCR 依赖）
- ✅ 识别率 > 90%
- ✅ 处理时间 < 3 秒/页
- ✅ 支持多平台（Android/iOS/Windows）
- ✅ 完善的文档和示例

