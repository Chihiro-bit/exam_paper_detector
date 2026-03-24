/// 题目框模型
///
/// 对应 Rust 侧的 QuestionBox 结构体
class QuestionBox {
  /// 页面索引
  final int pageIndex;

  /// 题号
  final String questionId;

  /// X 坐标
  final double x;

  /// Y 坐标
  final double y;

  /// 宽度
  final double width;

  /// 高度
  final double height;

  /// 题号锚点框（可能为空）
  final BoundingBox? titleAnchorBox;

  /// 置信度 (0.0-1.0)
  final double confidence;

  /// 识别到的题号文本
  final String? recognizedTitleText;

  /// 包含的 block ID 列表
  final List<int> blockIds;

  QuestionBox({
    this.pageIndex = 0,
    required this.questionId,
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    this.titleAnchorBox,
    required this.confidence,
    this.recognizedTitleText,
    this.blockIds = const [],
  });

  /// 从 Rust 侧 JSON 创建
  factory QuestionBox.fromJson(Map<String, dynamic> json) {
    final bbox = json['bounding_box'] as Map<String, dynamic>;

    Map<String, dynamic>? anchorJson;
    if (json['title_anchor_box'] != null) {
      anchorJson = json['title_anchor_box'] as Map<String, dynamic>;
    }

    return QuestionBox(
      pageIndex: json['page_index'] as int? ?? 0,
      questionId: json['question_id'] as String,
      x: (bbox['x'] as num).toDouble(),
      y: (bbox['y'] as num).toDouble(),
      width: (bbox['width'] as num).toDouble(),
      height: (bbox['height'] as num).toDouble(),
      titleAnchorBox: anchorJson != null
          ? BoundingBox.fromJson(anchorJson)
          : null,
      confidence: (json['confidence'] as num).toDouble(),
      recognizedTitleText: json['recognized_title_text'] as String?,
      blockIds: (json['block_ids'] as List?)
              ?.map((e) => e as int)
              .toList() ??
          [],
    );
  }

  /// 转换为 JSON
  Map<String, dynamic> toJson() {
    return {
      'page_index': pageIndex,
      'question_id': questionId,
      'bounding_box': {
        'x': x,
        'y': y,
        'width': width,
        'height': height,
      },
      if (titleAnchorBox != null)
        'title_anchor_box': titleAnchorBox!.toJson(),
      'confidence': confidence,
      'recognized_title_text': recognizedTitleText,
      'block_ids': blockIds,
    };
  }

  @override
  String toString() {
    return 'QuestionBox(id: $questionId, x: $x, y: $y, w: $width, h: $height, conf: ${confidence.toStringAsFixed(2)})';
  }
}

/// 边界框
class BoundingBox {
  final double x;
  final double y;
  final double width;
  final double height;

  BoundingBox({
    required this.x,
    required this.y,
    required this.width,
    required this.height,
  });

  factory BoundingBox.fromJson(Map<String, dynamic> json) {
    return BoundingBox(
      x: (json['x'] as num).toDouble(),
      y: (json['y'] as num).toDouble(),
      width: (json['width'] as num).toDouble(),
      height: (json['height'] as num).toDouble(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'x': x,
      'y': y,
      'width': width,
      'height': height,
    };
  }
}
