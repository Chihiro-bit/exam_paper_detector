import 'question_box.dart';

/// 检测结果
class DetectionResult {
  /// 是否成功
  final bool success;

  /// 检测到的题目数量
  final int questionCount;

  /// 处理时间（毫秒）
  final int processingTimeMs;

  /// 题目列表
  final List<QuestionBox> questions;

  /// 错误信息（如果有）
  final String? errorMessage;

  DetectionResult({
    required this.success,
    required this.questionCount,
    required this.processingTimeMs,
    required this.questions,
    this.errorMessage,
  });

  /// 从 Rust 侧返回的 JSON 创建
  ///
  /// Rust 侧 DetectionResult 的结构：
  /// ```json
  /// {
  ///   "status": "Success" | "PartialSuccess" | "Failed",
  ///   "questions": [...],
  ///   "metadata": { "total_questions": N, "processing_time_ms": N, ... },
  ///   "error": null | "..."
  /// }
  /// ```
  factory DetectionResult.fromJson(Map<String, dynamic> json) {
    final status = json['status'] as String?;
    final questions = (json['questions'] as List?)
            ?.map((q) => QuestionBox.fromJson(q as Map<String, dynamic>))
            .toList() ??
        [];

    final metadata = json['metadata'] as Map<String, dynamic>?;

    return DetectionResult(
      success: status == 'Success' || status == 'PartialSuccess',
      questionCount: metadata?['total_questions'] as int? ?? questions.length,
      processingTimeMs: metadata?['processing_time_ms'] as int? ?? 0,
      questions: questions,
      // Rust 侧字段名是 "error"，不是 "error_message"
      errorMessage: json['error'] as String?,
    );
  }

  /// 转换为 JSON
  Map<String, dynamic> toJson() {
    return {
      'status': success ? 'Success' : 'Failed',
      'questions': questions.map((q) => q.toJson()).toList(),
      'metadata': {
        'total_questions': questionCount,
        'processing_time_ms': processingTimeMs,
      },
      'error': errorMessage,
    };
  }

  @override
  String toString() {
    return 'DetectionResult(success: $success, questions: $questionCount, time: ${processingTimeMs}ms)';
  }
}
