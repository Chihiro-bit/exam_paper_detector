import 'dart:io';
import 'dart:ui' as ui;
import 'package:flutter/material.dart';
import 'package:file_picker/file_picker.dart';
import 'package:exam_paper_detector/exam_paper_detector.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Exam Paper Detector Demo',
      theme: ThemeData(
        primarySwatch: Colors.blue,
        useMaterial3: true,
      ),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  final DetectorService _detector = DetectorService();
  bool _initialized = false;
  String _status = '未初始化';
  DetectionResult? _lastResult;
  bool _processing = false;
  String? _selectedImagePath;
  ui.Image? _imageInfo;
  int? _selectedQuestionIndex;

  @override
  void initState() {
    super.initState();
    _initDetector();
  }

  Future<void> _initDetector() async {
    setState(() => _status = '正在初始化...');

    try {
      final success = await _detector.initialize();
      String version = 'unknown';
      if (success) version = await _detector.getVersion();

      setState(() {
        _initialized = success;
        _status = success ? '已初始化 (v$version)' : '初始化失败';
      });
    } catch (e) {
      setState(() => _status = '初始化错误: $e');
    }
  }

  Future<void> _pickAndProcessImage() async {
    if (!_initialized) {
      _showMessage('请先初始化检测器');
      return;
    }

    final result = await FilePicker.platform.pickFiles(
      type: FileType.image,
      allowMultiple: false,
    );

    if (result == null || result.files.isEmpty) return;
    final path = result.files.single.path;
    if (path == null) return;

    setState(() {
      _selectedImagePath = path;
      _processing = true;
      _status = '正在处理图片...';
      _lastResult = null;
      _imageInfo = null;
      _selectedQuestionIndex = null;
    });

    // 加载图片尺寸信息（用于坐标映射）
    final imageFile = File(path);
    final bytes = await imageFile.readAsBytes();
    final codec = await ui.instantiateImageCodec(bytes);
    final frame = await codec.getNextFrame();
    final loadedImage = frame.image;

    try {
      final detectionResult = await _detector.processImage(
        path,
        includeDebug: true,
      );

      setState(() {
        _lastResult = detectionResult;
        _imageInfo = loadedImage;
        _processing = false;
        _status = detectionResult.success
            ? '检测成功：找到 ${detectionResult.questionCount} 道题 (${detectionResult.processingTimeMs}ms)'
            : '检测失败：${detectionResult.errorMessage ?? "未知错误"}';
      });
    } catch (e) {
      setState(() {
        _processing = false;
        _status = '处理错误: $e';
      });
    }
  }

  void _showMessage(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message)),
    );
  }

  Color _questionColor(int index) {
    const colors = [
      Colors.red,
      Colors.blue,
      Colors.green,
      Colors.orange,
      Colors.purple,
      Colors.teal,
      Colors.pink,
      Colors.indigo,
      Colors.amber,
      Colors.cyan,
    ];
    return colors[index % colors.length];
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('试卷题目识别'),
        elevation: 2,
      ),
      body: Column(
        children: [
          // 顶部：状态 + 按钮
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
            child: Row(
              children: [
                Icon(
                  _initialized ? Icons.check_circle : Icons.error,
                  color: _initialized ? Colors.green : Colors.orange,
                  size: 20,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    _status,
                    style: TextStyle(
                      color: _initialized
                          ? Colors.green.shade700
                          : Colors.orange.shade700,
                      fontSize: 13,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 12),
                ElevatedButton.icon(
                  onPressed: _processing ? null : _pickAndProcessImage,
                  icon: _processing
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.image_search, size: 20),
                  label: Text(_processing ? '处理中...' : '选择图片'),
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),

          // 主体内容
          Expanded(
            child: _selectedImagePath == null
                ? const Center(
                    child: Text(
                      '点击右上角按钮选择试卷图片并开始检测',
                      style: TextStyle(color: Colors.grey),
                    ),
                  )
                : Row(
                    children: [
                      // 左侧：图片预览 + 检测框
                      Expanded(
                        flex: 3,
                        child: _buildImagePreview(),
                      ),
                      // 右侧：题目列表
                      if (_lastResult != null &&
                          _lastResult!.questions.isNotEmpty)
                        SizedBox(
                          width: 260,
                          child: _buildQuestionList(),
                        ),
                    ],
                  ),
          ),
        ],
      ),
    );
  }

  Widget _buildImagePreview() {
    return Container(
      margin: const EdgeInsets.fromLTRB(16, 0, 8, 16),
      decoration: BoxDecoration(
        border: Border.all(color: Colors.grey.shade300),
        borderRadius: BorderRadius.circular(8),
      ),
      clipBehavior: Clip.antiAlias,
      child: LayoutBuilder(
        builder: (context, constraints) {
          return InteractiveViewer(
            minScale: 0.5,
            maxScale: 5.0,
            child: Stack(
              fit: StackFit.expand,
              children: [
                // 图片
                Image.file(
                  File(_selectedImagePath!),
                  fit: BoxFit.contain,
                  alignment: Alignment.topCenter,
                ),
                // 检测框叠加层
                if (_lastResult != null &&
                    _lastResult!.questions.isNotEmpty &&
                    _imageInfo != null)
                  CustomPaint(
                    painter: QuestionBoxPainter(
                      questions: _lastResult!.questions,
                      imageWidth: _imageInfo!.width.toDouble(),
                      imageHeight: _imageInfo!.height.toDouble(),
                      containerSize: constraints.biggest,
                      questionColor: _questionColor,
                      selectedIndex: _selectedQuestionIndex,
                    ),
                  ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildQuestionList() {
    final questions = _lastResult!.questions;
    return Container(
      margin: const EdgeInsets.fromLTRB(0, 0, 16, 16),
      decoration: BoxDecoration(
        border: Border.all(color: Colors.grey.shade300),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Column(
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            decoration: BoxDecoration(
              color: Colors.grey.shade100,
              borderRadius:
                  const BorderRadius.vertical(top: Radius.circular(7)),
            ),
            child: Row(
              children: [
                const Icon(Icons.list_alt, size: 18),
                const SizedBox(width: 8),
                Text(
                  '检测到 ${questions.length} 道题',
                  style: const TextStyle(
                      fontWeight: FontWeight.bold, fontSize: 14),
                ),
              ],
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: ListView.builder(
              itemCount: questions.length,
              itemBuilder: (context, index) {
                final q = questions[index];
                final color = _questionColor(index);
                final isSelected = _selectedQuestionIndex == index;

                return InkWell(
                  onTap: () {
                    setState(() {
                      _selectedQuestionIndex =
                          isSelected ? null : index;
                    });
                  },
                  child: Container(
                    color: isSelected ? color.withValues(alpha: 0.1) : null,
                    padding: const EdgeInsets.symmetric(
                        horizontal: 12, vertical: 8),
                    child: Row(
                      children: [
                        Container(
                          width: 32,
                          height: 32,
                          decoration: BoxDecoration(
                            color: color.withValues(alpha: 0.15),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(
                              color: color,
                              width: isSelected ? 2 : 1,
                            ),
                          ),
                          alignment: Alignment.center,
                          child: Text(
                            q.questionId,
                            style: TextStyle(
                              color: color,
                              fontWeight: FontWeight.bold,
                              fontSize: 13,
                            ),
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                '题目 ${q.questionId}',
                                style: const TextStyle(
                                    fontWeight: FontWeight.w500,
                                    fontSize: 13),
                              ),
                              const SizedBox(height: 2),
                              Text(
                                '${q.width.toInt()}x${q.height.toInt()} '
                                '置信度 ${(q.confidence * 100).toStringAsFixed(0)}%',
                                style: TextStyle(
                                  color: Colors.grey.shade600,
                                  fontSize: 11,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _detector.dispose().then((_) {});
    super.dispose();
  }
}

/// 在图片上绘制题目检测框的 CustomPainter
class QuestionBoxPainter extends CustomPainter {
  final List<QuestionBox> questions;
  final double imageWidth;
  final double imageHeight;
  final Size containerSize;
  final Color Function(int index) questionColor;
  final int? selectedIndex;

  QuestionBoxPainter({
    required this.questions,
    required this.imageWidth,
    required this.imageHeight,
    required this.containerSize,
    required this.questionColor,
    this.selectedIndex,
  });

  @override
  void paint(Canvas canvas, Size size) {
    // 计算 BoxFit.contain 下图片的实际绘制区域
    final imageAspect = imageWidth / imageHeight;
    final containerAspect = containerSize.width / containerSize.height;

    double renderWidth, renderHeight;
    double offsetX, offsetY;

    if (imageAspect > containerAspect) {
      // 图片更宽，左右撑满
      renderWidth = containerSize.width;
      renderHeight = containerSize.width / imageAspect;
      offsetX = 0;
      offsetY = 0; // topCenter 对齐
    } else {
      // 图片更高，上下撑满或居上
      renderHeight = containerSize.height;
      renderWidth = containerSize.height * imageAspect;
      offsetX = (containerSize.width - renderWidth) / 2;
      offsetY = 0;
    }

    final scaleX = renderWidth / imageWidth;
    final scaleY = renderHeight / imageHeight;

    for (int i = 0; i < questions.length; i++) {
      final q = questions[i];
      final color = questionColor(i);
      final isSelected = selectedIndex == i;

      final rect = Rect.fromLTWH(
        offsetX + q.x * scaleX,
        offsetY + q.y * scaleY,
        q.width * scaleX,
        q.height * scaleY,
      );

      // 半透明填充
      final fillPaint = Paint()
        ..color = color.withValues(alpha: isSelected ? 0.15 : 0.08)
        ..style = PaintingStyle.fill;
      canvas.drawRect(rect, fillPaint);

      // 边框
      final strokePaint = Paint()
        ..color = color.withValues(alpha: isSelected ? 1.0 : 0.7)
        ..style = PaintingStyle.stroke
        ..strokeWidth = isSelected ? 3.0 : 2.0;
      canvas.drawRect(rect, strokePaint);

      // 题号标签
      final labelText = q.questionId;
      final textSpan = TextSpan(
        text: labelText,
        style: TextStyle(
          color: Colors.white,
          fontSize: isSelected ? 14 : 12,
          fontWeight: FontWeight.bold,
        ),
      );
      final textPainter = TextPainter(
        text: textSpan,
        textDirection: TextDirection.ltr,
      )..layout();

      final labelW = textPainter.width + 10;
      final labelH = textPainter.height + 6;
      final labelRect = Rect.fromLTWH(
        rect.left,
        rect.top - labelH,
        labelW,
        labelH,
      );

      // 标签背景
      final labelBgPaint = Paint()
        ..color = color.withValues(alpha: isSelected ? 1.0 : 0.85)
        ..style = PaintingStyle.fill;
      canvas.drawRRect(
        RRect.fromRectAndCorners(
          labelRect,
          topLeft: const Radius.circular(4),
          topRight: const Radius.circular(4),
        ),
        labelBgPaint,
      );

      textPainter.paint(
        canvas,
        Offset(labelRect.left + 5, labelRect.top + 3),
      );
    }
  }

  @override
  bool shouldRepaint(QuestionBoxPainter oldDelegate) {
    return oldDelegate.selectedIndex != selectedIndex ||
        oldDelegate.questions != questions ||
        oldDelegate.containerSize != containerSize;
  }
}
