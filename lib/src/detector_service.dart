import 'dart:convert';
import 'api.dart' as api;
import 'frb_generated.dart';
import 'models/detection_result.dart';
import 'models/detector_config.dart';
import 'models/question_box.dart';

/// 检测器服务
///
/// 提供简洁的 Dart API 来调用 Rust 核心功能。
/// 使用单例模式确保全局只有一个检测器实例。
class DetectorService {
  static DetectorService? _instance;
  bool _initialized = false;
  static bool _frbInitialized = false;

  DetectorService._();

  factory DetectorService() {
    _instance ??= DetectorService._();
    return _instance!;
  }

  bool get isInitialized => _initialized;

  /// 初始化 flutter_rust_bridge 运行时（只需调用一次）
  static Future<void> initFrb() async {
    if (_frbInitialized) return;
    await RustLib.init();
    _frbInitialized = true;
  }

  /// 初始化检测器
  Future<bool> initialize([DetectorConfig? config]) async {
    if (_initialized) return true;

    try {
      await initFrb();
      final configJson = config != null ? config.toJsonString() : '';
      final result = await api.initDetector(configJson: configJson);
      _initialized = result;
      return result;
    } catch (e) {
      log('Failed to initialize detector: $e');
      return false;
    }
  }

  /// 处理单张图片（推荐使用）
  Future<DetectionResult> processImage(
    String imagePath, {
    bool includeDebug = false,
  }) async {
    _ensureInitialized();

    try {
      final result = await api.processImageSimple(
        imagePath: imagePath,
        includeDebug: includeDebug,
      );

      return DetectionResult(
        success: result.success,
        questionCount: result.questionCount,
        processingTimeMs: result.processingTimeMs.toInt(),
        questions: result.questions
            .map((q) => QuestionBox(
                  questionId: q.questionId,
                  x: q.x,
                  y: q.y,
                  width: q.width,
                  height: q.height,
                  confidence: q.confidence,
                ))
            .toList(),
        errorMessage: result.errorMessage,
      );
    } catch (e) {
      log('Failed to process image: $e');
      return DetectionResult(
        success: false,
        questionCount: 0,
        processingTimeMs: 0,
        questions: [],
        errorMessage: e.toString(),
      );
    }
  }

  /// 处理单张图片（JSON API，用于需要完整结果的场景）
  Future<DetectionResult> processImageFull(
    String imagePath, {
    ProcessOptions? options,
  }) async {
    _ensureInitialized();

    try {
      final optionsJson = options != null ? jsonEncode(options.toJson()) : '';
      final resultJson = await api.processImage(
        imagePath: imagePath,
        optionsJson: optionsJson,
      );

      final resultMap = jsonDecode(resultJson) as Map<String, dynamic>;
      return DetectionResult.fromJson(resultMap);
    } catch (e) {
      log('Failed to process image: $e');
      return DetectionResult(
        success: false,
        questionCount: 0,
        processingTimeMs: 0,
        questions: [],
        errorMessage: e.toString(),
      );
    }
  }

  /// 批量处理多张图片
  Future<List<DetectionResult>> processBatch(
    List<String> imagePaths, {
    ProcessOptions? options,
  }) async {
    _ensureInitialized();

    try {
      final pathsJson = jsonEncode(imagePaths);
      final optionsJson = options != null ? jsonEncode(options.toJson()) : '';
      final resultsJson = await api.processBatch(
        imagePathsJson: pathsJson,
        optionsJson: optionsJson,
      );

      final resultsList = jsonDecode(resultsJson) as List;
      return resultsList
          .map((r) => DetectionResult.fromJson(r as Map<String, dynamic>))
          .toList();
    } catch (e) {
      log('Failed to process batch: $e');
      return [];
    }
  }

  /// 获取默认配置
  Future<DetectorConfig> getDefaultConfig() async {
    try {
      await initFrb();
      final configJson = await api.getDefaultConfig();
      final configMap = jsonDecode(configJson) as Map<String, dynamic>;
      return DetectorConfig.fromJson(configMap);
    } catch (e) {
      log('Failed to get default config: $e');
      return DetectorConfig.defaultConfig();
    }
  }

  /// 获取版本信息
  Future<String> getVersion() async {
    try {
      await initFrb();
      return await api.getVersion();
    } catch (e) {
      log('Failed to get version: $e');
      return 'unknown';
    }
  }

  /// 释放检测器资源
  Future<void> dispose() async {
    if (!_initialized) return;

    try {
      await api.disposeDetector();
      _initialized = false;
    } catch (e) {
      log('Failed to dispose detector: $e');
    }
  }

  void _ensureInitialized() {
    if (!_initialized) {
      throw StateError('Detector not initialized. Call initialize() first.');
    }
  }

  static void log(String message) {
    // ignore: avoid_print
    print('[ExamPaperDetector] $message');
  }
}

/// 处理选项
class ProcessOptions {
  final int pageIndex;
  final bool includeDebug;
  final bool saveIntermediate;

  ProcessOptions({
    this.pageIndex = 0,
    this.includeDebug = false,
    this.saveIntermediate = false,
  });

  Map<String, dynamic> toJson() {
    return {
      'page_index': pageIndex,
      'include_debug': includeDebug,
      'save_intermediate': saveIntermediate,
    };
  }
}
