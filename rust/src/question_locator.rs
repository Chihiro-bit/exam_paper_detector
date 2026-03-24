//! 题号定位模块
//!
//! 基于几何位置和 OCR 结果，定位题号位置

use crate::geometry::Rect;
use crate::ocr::OcrResult;
use crate::types::{PatternType, QuestionAnchor, QuestionPattern, TextBlock};
use regex::Regex;

/// 题号定位器
pub struct QuestionLocator {
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    regex: Regex,
    pattern_type: PatternType,
    priority: u8,
}

impl QuestionLocator {
    /// 创建定位器
    pub fn new(patterns: Vec<QuestionPattern>) -> anyhow::Result<Self> {
        let mut compiled_patterns = vec![];

        for pattern in patterns {
            match Regex::new(&pattern.pattern) {
                Ok(regex) => {
                    compiled_patterns.push(CompiledPattern {
                        regex,
                        pattern_type: pattern.pattern_type,
                        priority: pattern.priority,
                    });
                }
                Err(e) => {
                    log::warn!("Invalid regex pattern '{}': {}", pattern.pattern, e);
                }
            }
        }

        // 按优先级排序
        compiled_patterns.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(Self {
            patterns: compiled_patterns,
        })
    }

    /// 定位题号
    pub fn locate(
        &self,
        blocks: &[TextBlock],
        ocr_results: &[OcrResult],
        image_width: u32,
    ) -> anyhow::Result<Vec<QuestionAnchor>> {
        log::info!("Locating question numbers...");

        // Step 1: 基于 OCR 结果匹配题号
        let mut anchors = self.match_from_ocr(ocr_results)?;

        // Step 2: 基于几何位置推断题号（当 OCR 失败时）
        if anchors.is_empty() {
            log::warn!("No question numbers found from OCR, trying geometric inference");
            anchors = self.infer_from_geometry(blocks, image_width)?;
        }

        // Step 3: 验证题号序列
        anchors = self.validate_sequence(anchors);

        // Step 4: 按位置排序
        anchors.sort_by(|a, b| {
            a.bbox
                .y
                .partial_cmp(&b.bbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        log::info!("Located {} question numbers", anchors.len());
        Ok(anchors)
    }

    /// 从 OCR 结果中匹配题号
    fn match_from_ocr(&self, ocr_results: &[OcrResult]) -> anyhow::Result<Vec<QuestionAnchor>> {
        let mut anchors = vec![];

        for ocr_result in ocr_results {
            let text = ocr_result.text.trim();

            // 尝试所有模式
            for pattern in &self.patterns {
                if pattern.regex.is_match(text) {
                    // 提取题号
                    if let Some(question_id) = self.extract_question_id(text, pattern.pattern_type)
                    {
                        anchors.push(QuestionAnchor {
                            question_id,
                            bbox: ocr_result.bbox,
                            confidence: ocr_result.confidence,
                            text: text.to_string(),
                            pattern_type: pattern.pattern_type,
                        });
                        break; // 找到匹配就停止
                    }
                }
            }
        }

        Ok(anchors)
    }

    /// 提取题号 ID
    fn extract_question_id(&self, text: &str, pattern_type: PatternType) -> Option<String> {
        match pattern_type {
            PatternType::Numbered => {
                // "1." -> "1"
                text.trim_end_matches('.').parse::<u32>().ok().map(|n| n.to_string())
            }
            PatternType::Parenthesized => {
                // "(1)" -> "1"
                text.trim_start_matches('(')
                    .trim_end_matches(')')
                    .parse::<u32>()
                    .ok()
                    .map(|n| n.to_string())
            }
            PatternType::Chinese => {
                // "一、" -> "1"
                self.chinese_to_number(text.trim_end_matches('、'))
            }
            PatternType::Bracketed => {
                // "【1】" -> "1"
                text.trim_start_matches('【')
                    .trim_end_matches('】')
                    .parse::<u32>()
                    .ok()
                    .map(|n| n.to_string())
            }
        }
    }

    /// 中文数字转阿拉伯数字
    fn chinese_to_number(&self, text: &str) -> Option<String> {
        let digit_map: std::collections::HashMap<char, u32> = [
            ('零', 0),
            ('一', 1),
            ('二', 2),
            ('三', 3),
            ('四', 4),
            ('五', 5),
            ('六', 6),
            ('七', 7),
            ('八', 8),
            ('九', 9),
        ]
        .iter()
        .cloned()
        .collect();

        let chars: Vec<char> = text.chars().collect();

        match chars.len() {
            1 => {
                if chars[0] == '十' {
                    return Some("10".to_string());
                }
                digit_map.get(&chars[0]).map(|n| n.to_string())
            }
            2 => {
                if chars[0] == '十' {
                    digit_map.get(&chars[1]).map(|n| (10 + n).to_string())
                } else if chars[1] == '十' {
                    digit_map.get(&chars[0]).map(|n| (n * 10).to_string())
                } else {
                    None
                }
            }
            3 => {
                if chars[1] == '十' {
                    let tens = digit_map.get(&chars[0])?;
                    let ones = digit_map.get(&chars[2])?;
                    Some((tens * 10 + ones).to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 基于几何位置推断题号
    ///
    /// 核心思想：每一道题的起始行，其行首 block 的 x 坐标相对靠左，
    /// 与同一道题内的续行（缩进行）相比更靠近左边缘。
    /// 通过检测"行首缩进突变"来识别题目边界。
    fn infer_from_geometry(
        &self,
        blocks: &[TextBlock],
        image_width: u32,
    ) -> anyhow::Result<Vec<QuestionAnchor>> {
        if blocks.is_empty() {
            return Ok(vec![]);
        }

        // 按行分组（y 坐标相近的 blocks 归为同一行）
        let lines = self.group_blocks_into_lines(blocks);

        if lines.is_empty() {
            return Ok(vec![]);
        }

        // 对每一行，计算行首 x 坐标（最左 block 的 x）
        let line_info: Vec<LineInfo> = lines
            .iter()
            .map(|line_blocks| {
                let min_x = line_blocks.iter().map(|b| b.bbox.x).fold(f64::MAX, f64::min);
                let min_y = line_blocks.iter().map(|b| b.bbox.y).fold(f64::MAX, f64::min);
                let max_right = line_blocks
                    .iter()
                    .map(|b| b.bbox.x + b.bbox.width)
                    .fold(0.0, f64::max);
                let first_block_width = line_blocks
                    .iter()
                    .min_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap())
                    .map(|b| b.bbox.width)
                    .unwrap_or(0.0);
                LineInfo {
                    x_start: min_x,
                    y: min_y,
                    width: max_right - min_x,
                    first_block_width,
                    blocks: line_blocks.clone(),
                }
            })
            .collect();

        log::debug!("Grouped into {} lines", line_info.len());

        // 找出全局最左缩进级别（题号行的 x）
        // 通常试卷中题号行的 x 坐标最为一致（左对齐）
        let mut x_starts: Vec<f64> = line_info.iter().map(|l| l.x_start).collect();
        x_starts.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // 找最常见的左缩进 x（聚类）
        let left_margin_x = self.find_dominant_x(&x_starts, image_width as f64 * 0.05);

        log::debug!("Dominant left margin x: {:.0}", left_margin_x);

        // 收集"题号行候选"：行首 x 接近 left_margin_x 的行
        let x_tolerance = image_width as f64 * 0.08; // 8% 容差

        // 计算行间距，用"自然断点"区分续行和新题目
        let mut line_gaps: Vec<f64> = vec![];
        for i in 1..line_info.len() {
            line_gaps.push(line_info[i].y - line_info[i - 1].y);
        }
        let min_question_gap = self.find_gap_threshold(&line_gaps);

        let mut anchors = vec![];
        let mut question_id = 1u32;
        let mut last_anchor_y: Option<f64> = None;

        for info in &line_info {
            // 判断是否是题号行：
            // 1. 行首 x 接近全局左边缘
            let is_near_left = (info.x_start - left_margin_x).abs() < x_tolerance;

            // 排除过短的行（可能是公式残片或独立符号）
            let is_substantial = info.width > image_width as f64 * 0.15
                || info.first_block_width < 60.0;

            // 排除标题行特征：居中且不从左侧边缘开始的行
            // 标题行的特点：中心接近页面中心，且起始位置不在左边缘
            let center_x = info.x_start + info.width / 2.0;
            let is_centered = (center_x - image_width as f64 / 2.0).abs() < image_width as f64 * 0.15
                && info.x_start > image_width as f64 * 0.15;

            if is_near_left && is_substantial && !is_centered {
                // 检查与上一个 anchor 的间距——太近则视为续行，跳过
                let is_continuation = match last_anchor_y {
                    Some(prev_y) => (info.y - prev_y) < min_question_gap,
                    None => false,
                };

                if is_continuation {
                    log::debug!(
                        "Skipping continuation line at y={:.0} (gap={:.0} < {:.0})",
                        info.y,
                        info.y - last_anchor_y.unwrap_or(0.0),
                        min_question_gap
                    );
                    continue;
                }

                // 使用行首的第一个小 block 作为 anchor
                let anchor_block = info
                    .blocks
                    .iter()
                    .min_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap())
                    .unwrap();

                anchors.push(QuestionAnchor {
                    question_id: question_id.to_string(),
                    bbox: Rect::new(
                        anchor_block.bbox.x,
                        info.y,
                        anchor_block.bbox.width.min(40.0),
                        anchor_block.bbox.height,
                    ),
                    confidence: 0.6,
                    text: format!("{}.", question_id),
                    pattern_type: PatternType::Numbered,
                });
                last_anchor_y = Some(info.y);
                question_id += 1;
            }
        }

        // 如果检测到太少或太多，尝试回退策略
        if anchors.len() < 3 {
            log::warn!(
                "Too few anchors found ({}), trying fallback line-gap analysis",
                anchors.len()
            );
            anchors = self.infer_from_line_gaps(&line_info, image_width)?;
        }

        // 过滤可能的章节标题行（块高度明显大于平均值的行）
        anchors = self.filter_section_headers(anchors, blocks);

        Ok(anchors)
    }

    /// 将 blocks 按行分组
    fn group_blocks_into_lines<'a>(&self, blocks: &'a [TextBlock]) -> Vec<Vec<&'a TextBlock>> {
        if blocks.is_empty() {
            return vec![];
        }

        let mut sorted: Vec<&TextBlock> = blocks.iter().collect();
        sorted.sort_by(|a, b| a.bbox.y.partial_cmp(&b.bbox.y).unwrap());

        // 计算典型行高
        let avg_height = blocks.iter().map(|b| b.bbox.height).sum::<f64>() / blocks.len() as f64;
        let line_tolerance = avg_height * 0.7;

        let mut lines: Vec<Vec<&TextBlock>> = vec![];
        let mut current_line: Vec<&TextBlock> = vec![sorted[0]];
        let mut current_y = sorted[0].bbox.y;

        for block in sorted.iter().skip(1) {
            if (block.bbox.y - current_y).abs() <= line_tolerance {
                current_line.push(block);
            } else {
                lines.push(current_line);
                current_line = vec![block];
                current_y = block.bbox.y;
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// 找到间距的自然断点阈值
    ///
    /// 将行间距排序后，找到相邻值之间的第一个"显著跳变"，
    /// 用其中点作为阈值来区分"续行间距"和"题间距"。
    ///
    /// 例如排序后的间距 [27, 28, 40, 41, 41, 59, 100]
    /// 第一个显著跳变是 28→40（差值12），阈值=34
    fn find_gap_threshold(&self, gaps: &[f64]) -> f64 {
        if gaps.is_empty() {
            return 20.0;
        }

        let mut sorted = gaps.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        if sorted.len() < 2 {
            return sorted[0] * 0.8;
        }

        // 计算相邻值的跳变量
        let mut jumps: Vec<(f64, f64, f64)> = vec![]; // (jump_size, low, high)
        for i in 1..sorted.len() {
            let jump = sorted[i] - sorted[i - 1];
            if jump > 0.0 {
                jumps.push((jump, sorted[i - 1], sorted[i]));
            }
        }

        if jumps.is_empty() {
            // 所有间距相同 — 无法区分续行和新题，不做续行过滤
            return sorted[0] * 0.8;
        }

        // 显著跳变的判定：跳变量 > 最小间距值的 30%，且跳变量 > 5px
        let min_gap = sorted[0];
        let significance_threshold = (min_gap * 0.3).max(5.0);

        // 找第一个显著跳变（从小到大方向）
        let threshold = jumps
            .iter()
            .find(|(jump_size, _, _)| *jump_size >= significance_threshold)
            .map(|(_, low, high)| (low + high) / 2.0)
            .unwrap_or(sorted[sorted.len() / 2] * 0.8);

        log::debug!(
            "Gap threshold: {:.1} (gaps: {:?}, min_gap={:.1}, significance={:.1})",
            threshold,
            sorted.iter().map(|v| *v as i32).collect::<Vec<_>>(),
            min_gap,
            significance_threshold
        );

        threshold.max(15.0)
    }

    /// 找出最常出现的 x 坐标（聚类中心）
    fn find_dominant_x(&self, sorted_xs: &[f64], tolerance: f64) -> f64 {
        if sorted_xs.is_empty() {
            return 0.0;
        }

        let mut best_x = sorted_xs[0];
        let mut best_count = 0;

        for &x in sorted_xs {
            let count = sorted_xs.iter().filter(|&&v| (v - x).abs() < tolerance).count();
            if count > best_count {
                best_count = count;
                best_x = x;
            }
        }

        // 返回该聚类的平均值
        let cluster: Vec<f64> = sorted_xs
            .iter()
            .filter(|&&v| (v - best_x).abs() < tolerance)
            .copied()
            .collect();
        cluster.iter().sum::<f64>() / cluster.len() as f64
    }

    /// 过滤可能的章节标题行
    ///
    /// 章节标题（如"一、仔细想..."、"二、小小裁判员..."）通常具有以下特征：
    /// 1. 块高度明显大于正文（使用更大的字号）
    /// 2. 位于文档的大段落分隔处
    fn filter_section_headers(
        &self,
        anchors: Vec<QuestionAnchor>,
        blocks: &[TextBlock],
    ) -> Vec<QuestionAnchor> {
        if anchors.len() < 3 || blocks.is_empty() {
            return anchors;
        }

        // 计算全局块高度中位数
        let mut all_heights: Vec<f64> = blocks.iter().map(|b| b.bbox.height).collect();
        all_heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_height = all_heights[all_heights.len() / 2];

        // 计算 anchor 间距的中位数
        let mut gaps: Vec<f64> = anchors
            .windows(2)
            .map(|w| w[1].bbox.y - w[0].bbox.y)
            .collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_gap = if gaps.is_empty() {
            40.0
        } else {
            gaps[gaps.len() / 2]
        };

        let mut result = vec![];

        for (i, anchor) in anchors.iter().enumerate() {
            // 找到该 anchor 行的所有 blocks
            let line_blocks: Vec<&TextBlock> = blocks
                .iter()
                .filter(|b| (b.bbox.y - anchor.bbox.y).abs() < median_height * 0.8)
                .collect();

            if line_blocks.is_empty() {
                result.push(anchor.clone());
                continue;
            }

            let line_avg_height =
                line_blocks.iter().map(|b| b.bbox.height).sum::<f64>() / line_blocks.len() as f64;

            // 特征1：该行的块高度明显大于中位数（大字号章节标题）
            let is_tall = line_avg_height > median_height * 1.1;

            // 特征2：该行前方有大间距（段落分隔）
            let gap_before = if i > 0 {
                anchor.bbox.y - anchors[i - 1].bbox.y
            } else {
                0.0
            };
            let has_large_gap_before = gap_before > median_gap * 2.0;

            // 特征3：是否是第一个 anchor（第一个 anchor 如果是高块，很可能是章节标题）
            let is_first = i == 0;

            // 判定为章节标题的条件（保守策略，避免误删真实题目）：
            // - 块高度偏大 AND（是第一个 anchor 或前方有大间距）
            // - 或者：位于大间距之后，且该行块数量较多（标题描述较长）
            let is_section_header = (is_tall && (is_first || has_large_gap_before))
                || (has_large_gap_before && line_blocks.len() > 15);

            if is_section_header {
                log::info!(
                    "Filtering section header at y={:.0} (avg_h={:.1} vs median={:.1}, gap_before={:.0}, blocks={})",
                    anchor.bbox.y,
                    line_avg_height,
                    median_height,
                    gap_before,
                    line_blocks.len()
                );
            } else {
                result.push(anchor.clone());
            }
        }

        // 重新编号
        for (i, anchor) in result.iter_mut().enumerate() {
            anchor.question_id = (i + 1).to_string();
            anchor.text = format!("{}.", i + 1);
        }

        log::info!(
            "After section header filtering: {} -> {} anchors",
            anchors.len(),
            result.len()
        );
        result
    }

    /// 基于行间距变化推断题目边界
    ///
    /// 当题目之间有额外的垂直间距时，利用间距突变来定位
    fn infer_from_line_gaps(
        &self,
        lines: &[LineInfo],
        _image_width: u32,
    ) -> anyhow::Result<Vec<QuestionAnchor>> {
        if lines.len() < 2 {
            return Ok(vec![]);
        }

        // 计算相邻行间距
        let mut gaps: Vec<f64> = vec![];
        for i in 1..lines.len() {
            gaps.push(lines[i].y - lines[i - 1].y);
        }

        // 计算中位间距
        let mut sorted_gaps = gaps.clone();
        sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_gap = sorted_gaps[sorted_gaps.len() / 2];

        // 间距显著大于中位值的位置是题目边界
        let gap_threshold = median_gap * 1.5;

        let mut anchors = vec![];
        let mut question_id = 1u32;

        // 第一行总是第一道题（或标题，但先加入后面再验证）
        anchors.push(self.create_anchor_from_line(&lines[0], question_id));
        question_id += 1;

        for i in 0..gaps.len() {
            if gaps[i] > gap_threshold {
                // 间距突变 = 新题目开始
                anchors.push(self.create_anchor_from_line(&lines[i + 1], question_id));
                question_id += 1;
            }
        }

        Ok(anchors)
    }

    fn create_anchor_from_line(&self, line: &LineInfo, question_id: u32) -> QuestionAnchor {
        let first_block = line
            .blocks
            .iter()
            .min_by(|a, b| a.bbox.x.partial_cmp(&b.bbox.x).unwrap())
            .unwrap();

        QuestionAnchor {
            question_id: question_id.to_string(),
            bbox: Rect::new(
                first_block.bbox.x,
                line.y,
                first_block.bbox.width.min(40.0),
                first_block.bbox.height,
            ),
            confidence: 0.5,
            text: format!("{}.", question_id),
            pattern_type: PatternType::Numbered,
        }
    }

    /// 验证题号序列
    fn validate_sequence(&self, mut anchors: Vec<QuestionAnchor>) -> Vec<QuestionAnchor> {
        if anchors.is_empty() {
            return anchors;
        }

        // 按位置排序
        anchors.sort_by(|a, b| {
            a.bbox
                .y
                .partial_cmp(&b.bbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 检查序列连续性
        let mut valid_anchors = vec![anchors[0].clone()];

        for i in 1..anchors.len() {
            let prev_id: u32 = valid_anchors
                .last()
                .unwrap()
                .question_id
                .parse()
                .unwrap_or(0);
            let current_id: u32 = anchors[i].question_id.parse().unwrap_or(0);

            // 允许跳号或连续
            if current_id > prev_id {
                valid_anchors.push(anchors[i].clone());
            } else {
                log::warn!(
                    "Skipping non-sequential question number: {} after {}",
                    current_id,
                    prev_id
                );
            }
        }

        valid_anchors
    }
}

#[derive(Debug, Clone)]
struct LineInfo<'a> {
    x_start: f64,
    y: f64,
    width: f64,
    first_block_width: f64,
    blocks: Vec<&'a TextBlock>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_question_id() {
        let locator = QuestionLocator::new(vec![]).unwrap();

        assert_eq!(
            locator.extract_question_id("1.", PatternType::Numbered),
            Some("1".to_string())
        );
        assert_eq!(
            locator.extract_question_id("(5)", PatternType::Parenthesized),
            Some("5".to_string())
        );
        assert_eq!(
            locator.extract_question_id("【10】", PatternType::Bracketed),
            Some("10".to_string())
        );
    }

    #[test]
    fn test_chinese_to_number() {
        let locator = QuestionLocator::new(vec![]).unwrap();

        assert_eq!(locator.chinese_to_number("一"), Some("1".to_string()));
        assert_eq!(locator.chinese_to_number("五"), Some("5".to_string()));
        assert_eq!(locator.chinese_to_number("十"), Some("10".to_string()));
        assert_eq!(locator.chinese_to_number("十一"), Some("11".to_string()));
        assert_eq!(locator.chinese_to_number("十五"), Some("15".to_string()));
        assert_eq!(locator.chinese_to_number("二十"), Some("20".to_string()));
        assert_eq!(locator.chinese_to_number("二十一"), Some("21".to_string()));
        assert_eq!(locator.chinese_to_number("三十五"), Some("35".to_string()));
        assert_eq!(locator.chinese_to_number("ABC"), None);
    }
}
