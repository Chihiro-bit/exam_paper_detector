import 'dart:convert';

/// 二值化方法（对应 Rust BinarizationMethod 枚举）
enum BinarizationMethod {
  otsu('Otsu'),
  adaptive('Adaptive'),
  fixed('Fixed');

  final String value;
  const BinarizationMethod(this.value);

  factory BinarizationMethod.fromString(String s) {
    return BinarizationMethod.values.firstWhere(
      (e) => e.value == s,
      orElse: () => BinarizationMethod.adaptive,
    );
  }
}

/// 题号模式类型（对应 Rust PatternType 枚举）
enum PatternType {
  numbered('Numbered'),
  parenthesized('Parenthesized'),
  chinese('Chinese'),
  bracketed('Bracketed');

  final String value;
  const PatternType(this.value);

  factory PatternType.fromString(String s) {
    return PatternType.values.firstWhere(
      (e) => e.value == s,
      orElse: () => PatternType.numbered,
    );
  }
}

/// OCR 引擎类型（对应 Rust OcrEngine 枚举）
enum OcrEngineType {
  tesseract('Tesseract'),
  mock('Mock');

  final String value;
  const OcrEngineType(this.value);

  factory OcrEngineType.fromString(String s) {
    return OcrEngineType.values.firstWhere(
      (e) => e.value == s,
      orElse: () => OcrEngineType.mock,
    );
  }
}

/// 检测器配置
class DetectorConfig {
  /// 预处理配置
  final PreprocessingConfig preprocessing;

  /// 题号模式
  final List<QuestionPattern> questionPatterns;

  /// OCR 配置（可选）
  final OcrConfig? ocr;

  /// Debug 配置
  final DebugConfig debug;

  DetectorConfig({
    required this.preprocessing,
    required this.questionPatterns,
    this.ocr,
    required this.debug,
  });

  /// 默认配置
  factory DetectorConfig.defaultConfig() {
    return DetectorConfig(
      preprocessing: PreprocessingConfig.defaultConfig(),
      questionPatterns: QuestionPattern.defaultPatterns(),
      ocr: null,
      debug: DebugConfig.defaultConfig(),
    );
  }

  /// 从 JSON 创建
  factory DetectorConfig.fromJson(Map<String, dynamic> json) {
    return DetectorConfig(
      preprocessing: PreprocessingConfig.fromJson(
          json['preprocessing'] as Map<String, dynamic>),
      questionPatterns: (json['question_patterns'] as List)
          .map((p) => QuestionPattern.fromJson(p as Map<String, dynamic>))
          .toList(),
      ocr: json['ocr'] != null
          ? OcrConfig.fromJson(json['ocr'] as Map<String, dynamic>)
          : null,
      debug: DebugConfig.fromJson(json['debug'] as Map<String, dynamic>),
    );
  }

  /// 转换为 JSON（用于传递给 Rust 侧）
  Map<String, dynamic> toJson() {
    return {
      'preprocessing': preprocessing.toJson(),
      'question_patterns': questionPatterns.map((p) => p.toJson()).toList(),
      if (ocr != null) 'ocr': ocr!.toJson(),
      'debug': debug.toJson(),
    };
  }

  /// 序列化为 JSON 字符串
  String toJsonString() => jsonEncode(toJson());
}

/// 预处理配置
class PreprocessingConfig {
  final bool enableDeskew;
  final bool enableDenoise;
  final BinarizationMethod binarizationMethod;
  final double contrastEnhancement;

  PreprocessingConfig({
    required this.enableDeskew,
    required this.enableDenoise,
    required this.binarizationMethod,
    required this.contrastEnhancement,
  });

  factory PreprocessingConfig.defaultConfig() {
    return PreprocessingConfig(
      enableDeskew: true,
      enableDenoise: true,
      binarizationMethod: BinarizationMethod.adaptive,
      contrastEnhancement: 1.2,
    );
  }

  factory PreprocessingConfig.fromJson(Map<String, dynamic> json) {
    return PreprocessingConfig(
      enableDeskew: json['enable_deskew'] as bool,
      enableDenoise: json['enable_denoise'] as bool,
      binarizationMethod:
          BinarizationMethod.fromString(json['binarization_method'] as String),
      contrastEnhancement: (json['contrast_enhancement'] as num).toDouble(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'enable_deskew': enableDeskew,
      'enable_denoise': enableDenoise,
      'binarization_method': binarizationMethod.value,
      'contrast_enhancement': contrastEnhancement,
    };
  }
}

/// 题号模式
class QuestionPattern {
  final String pattern;
  final PatternType patternType;
  final int priority;

  QuestionPattern({
    required this.pattern,
    required this.patternType,
    required this.priority,
  });

  /// 默认模式库
  static List<QuestionPattern> defaultPatterns() {
    return [
      QuestionPattern(
        pattern: r'^\d+\.',
        patternType: PatternType.numbered,
        priority: 10,
      ),
      QuestionPattern(
        pattern: r'^\(\d+\)',
        patternType: PatternType.parenthesized,
        priority: 9,
      ),
      QuestionPattern(
        pattern: r'^[一二三四五六七八九十百]+、',
        patternType: PatternType.chinese,
        priority: 8,
      ),
      QuestionPattern(
        pattern: r'^【\d+】',
        patternType: PatternType.bracketed,
        priority: 7,
      ),
    ];
  }

  factory QuestionPattern.fromJson(Map<String, dynamic> json) {
    return QuestionPattern(
      pattern: json['pattern'] as String,
      patternType: PatternType.fromString(json['pattern_type'] as String),
      priority: json['priority'] as int,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'pattern': pattern,
      'pattern_type': patternType.value,
      'priority': priority,
    };
  }
}

/// OCR 配置
class OcrConfig {
  final OcrEngineType engine;
  final String language;
  final double confidenceThreshold;

  OcrConfig({
    required this.engine,
    required this.language,
    required this.confidenceThreshold,
  });

  factory OcrConfig.fromJson(Map<String, dynamic> json) {
    return OcrConfig(
      engine: OcrEngineType.fromString(json['engine'] as String),
      language: json['language'] as String,
      confidenceThreshold: (json['confidence_threshold'] as num).toDouble(),
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'engine': engine.value,
      'language': language,
      'confidence_threshold': confidenceThreshold,
    };
  }
}

/// Debug 配置
class DebugConfig {
  final bool saveIntermediate;
  final String? outputDir;
  final bool verbose;

  DebugConfig({
    required this.saveIntermediate,
    this.outputDir,
    required this.verbose,
  });

  factory DebugConfig.defaultConfig() {
    return DebugConfig(
      saveIntermediate: false,
      outputDir: null,
      verbose: false,
    );
  }

  factory DebugConfig.fromJson(Map<String, dynamic> json) {
    return DebugConfig(
      saveIntermediate: json['save_intermediate'] as bool,
      outputDir: json['output_dir'] as String?,
      verbose: json['verbose'] as bool,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'save_intermediate': saveIntermediate,
      if (outputDir != null) 'output_dir': outputDir,
      'verbose': verbose,
    };
  }
}
