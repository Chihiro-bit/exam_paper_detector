//! 题目分段模块
//!
//! 基于题号锚点，将文本块分组为完整的题目

use crate::geometry::Rect;
use crate::types::{QuestionAnchor, QuestionBox, QuestionDebugInfo, TextBlock};

/// 题目分段器
pub struct QuestionSegmenter {}

impl QuestionSegmenter {
    pub fn new() -> Self {
        Self {}
    }

    /// 分割题目
    pub fn segment(
        &self,
        blocks: &[TextBlock],
        anchors: &[QuestionAnchor],
        include_debug: bool,
    ) -> anyhow::Result<Vec<QuestionBox>> {
        log::info!("Segmenting questions...");

        if anchors.is_empty() {
            log::warn!("No anchors provided, cannot segment questions");
            return Ok(vec![]);
        }

        let mut questions = vec![];

        for (i, anchor) in anchors.iter().enumerate() {
            // 确定题目的范围
            let y_start = anchor.bbox.y;
            let y_end = if i + 1 < anchors.len() {
                anchors[i + 1].bbox.y
            } else {
                // 最后一题：延伸到最后一个 block
                blocks
                    .iter()
                    .map(|b| b.bbox.y + b.bbox.height)
                    .fold(0.0, f64::max)
            };

            // 查找属于本题的所有 blocks
            let question_blocks = self.find_question_blocks(blocks, anchor, y_start, y_end);

            if question_blocks.is_empty() {
                log::warn!("No blocks found for question {}", anchor.question_id);
                continue;
            }

            // 计算题目边界框（使用完整的垂直范围，覆盖到下一题之前）
            let margin = if i + 1 < anchors.len() { 3.0 } else { 0.0 };
            let bounding_box = self.calculate_bounding_box(&question_blocks, anchor, y_end - margin);

            // 计算置信度
            let confidence = self.calculate_confidence(anchor, &question_blocks, y_start, y_end);

            // 生成 Debug 信息
            let debug_info = if include_debug {
                Some(QuestionDebugInfo {
                    num_blocks: question_blocks.len(),
                    detection_method: "anchor_based".to_string(),
                    has_options: self.has_options(&question_blocks),
                    has_image: false, // TODO: 检测图片区域
                })
            } else {
                None
            };

            questions.push(QuestionBox {
                page_index: 0,
                question_id: anchor.question_id.clone(),
                bounding_box,
                title_anchor_box: Some(anchor.bbox),
                confidence,
                recognized_title_text: Some(anchor.text.clone()),
                block_ids: question_blocks.iter().map(|b| b.id).collect(),
                debug_info,
            });
        }

        log::info!("Segmented {} questions", questions.len());
        Ok(questions)
    }

    /// 查找属于指定题目的 blocks
    fn find_question_blocks(
        &self,
        all_blocks: &[TextBlock],
        anchor: &QuestionAnchor,
        y_start: f64,
        y_end: f64,
    ) -> Vec<TextBlock> {
        let mut question_blocks = vec![];

        // 计算锚点所在列的水平范围限制
        // 允许 blocks 在锚点 x 位置 2 倍范围内（从左边界算起）
        let x_limit = anchor.bbox.x * 2.0 + anchor.bbox.width;

        for block in all_blocks {
            // 使用 block 的垂直中心来判断归属，处理跨边界的 blocks
            let block_center_y = block.bbox.y + block.bbox.height / 2.0;

            if block_center_y >= y_start && block_center_y < y_end {
                // x-range 验证：block 应与锚点的列区域水平重叠
                let block_right = block.bbox.x + block.bbox.width;
                if block.bbox.x < x_limit && block_right > 0.0 {
                    question_blocks.push(block.clone());
                }
            } else if block.bbox.y >= y_start && block.bbox.y < y_end {
                // 对于垂直中心不在范围内但起始位置在范围内的 block，
                // 检查是否为缩进的子题（如 (1), (2), ①②③）
                if self.is_sub_question(block) {
                    let block_right = block.bbox.x + block.bbox.width;
                    if block.bbox.x < x_limit && block_right > 0.0 {
                        question_blocks.push(block.clone());
                    }
                }
            }
        }

        // 按 y 坐标排序
        question_blocks.sort_by(|a, b| {
            a.bbox
                .y
                .partial_cmp(&b.bbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        question_blocks
    }

    /// 检测 block 是否为缩进的子题目
    fn is_sub_question(&self, block: &TextBlock) -> bool {
        if let Some(ref text) = block.text {
            let trimmed = text.trim();
            // 匹配 (1), (2), ... 格式
            if trimmed.starts_with('(') && trimmed.len() >= 3 {
                let inner = &trimmed[1..];
                if let Some(pos) = inner.find(')') {
                    let num_part = &inner[..pos];
                    if num_part.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                }
            }
            // 匹配 ①②③④⑤⑥⑦⑧⑨⑩ 等圆圈数字
            if trimmed.starts_with(|c: char| {
                ('\u{2460}'..='\u{2473}').contains(&c) // ①-⑳
                    || ('\u{3251}'..='\u{325F}').contains(&c) // ㉑-㉟
            }) {
                return true;
            }
        }
        false
    }

    /// 计算题目边界框
    ///
    /// 使用完整的垂直范围（从当前锚点到下一个锚点），
    /// 水平范围覆盖所有 blocks 的最大宽度。
    fn calculate_bounding_box(
        &self,
        blocks: &[TextBlock],
        anchor: &QuestionAnchor,
        y_end: f64,
    ) -> Rect {
        if blocks.is_empty() {
            return Rect::new(anchor.bbox.x, anchor.bbox.y, anchor.bbox.width, y_end - anchor.bbox.y);
        }

        // 水平范围：取所有 blocks 和 anchor 的最大范围
        let min_x = blocks
            .iter()
            .map(|b| b.bbox.x)
            .fold(f64::MAX, f64::min)
            .min(anchor.bbox.x);
        let max_right = blocks
            .iter()
            .map(|b| b.bbox.x + b.bbox.width)
            .fold(0.0, f64::max)
            .max(anchor.bbox.x + anchor.bbox.width);

        // 垂直范围：从 anchor 的 y 到 y_end（下一题开始前）
        let y_start = anchor.bbox.y;
        let height = (y_end - y_start).max(anchor.bbox.height);

        Rect::new(min_x, y_start, max_right - min_x, height)
    }

    /// 计算置信度
    fn calculate_confidence(
        &self,
        anchor: &QuestionAnchor,
        blocks: &[TextBlock],
        y_start: f64,
        y_end: f64,
    ) -> f64 {
        // 综合评分：
        // - 题号识别置信度 × 0.4
        // - 几何一致性 × 0.3
        // - Block 数量合理性 × 0.2
        // - 垂直覆盖率 × 0.1

        let anchor_confidence = anchor.confidence * 0.4;

        // 几何一致性：blocks 应该基本左对齐
        let geometric_score = self.calculate_geometric_consistency(blocks) * 0.3;

        // Block 数量合理性：1-20 个 blocks 为合理
        let block_count_score = if blocks.len() >= 1 && blocks.len() <= 20 {
            1.0
        } else if blocks.len() > 20 {
            0.5
        } else {
            0.3
        } * 0.2;

        // 垂直覆盖率：blocks 高度之和 / 题目区域高度
        let region_height = (y_end - y_start).max(1.0);
        let total_block_height: f64 = blocks.iter().map(|b| b.bbox.height).sum();
        let coverage_ratio = total_block_height / region_height;
        let coverage_score = (coverage_ratio * 1.2).min(1.0) * 0.1;

        anchor_confidence + geometric_score + block_count_score + coverage_score
    }

    /// 计算几何一致性
    fn calculate_geometric_consistency(&self, blocks: &[TextBlock]) -> f64 {
        if blocks.len() < 2 {
            return 1.0;
        }

        // 计算左边界的标准差
        let x_values: Vec<f64> = blocks.iter().map(|b| b.bbox.x).collect();
        let mean_x = x_values.iter().sum::<f64>() / x_values.len() as f64;
        let variance = x_values
            .iter()
            .map(|x| (x - mean_x).powi(2))
            .sum::<f64>()
            / x_values.len() as f64;
        let std_dev = variance.sqrt();

        // 标准差越小，对齐越好
        if std_dev < 10.0 {
            1.0
        } else if std_dev < 30.0 {
            0.8
        } else if std_dev < 50.0 {
            0.6
        } else {
            0.4
        }
    }

    /// 检测是否有选项
    fn has_options(&self, blocks: &[TextBlock]) -> bool {
        // 简化检测：查找包含 "A" "B" "C" "D" 等字母的小块
        blocks.iter().any(|b| {
            if let Some(ref text) = b.text {
                let trimmed = text.trim();
                matches!(
                    trimmed,
                    "A" | "B" | "C" | "D" | "E" | "F" | "A." | "B." | "C." | "D."
                )
            } else {
                false
            }
        })
    }

    /// 回退模式：当没有题号时，按段落分割
    pub fn fallback_segment(&self, blocks: &[TextBlock]) -> anyhow::Result<Vec<QuestionBox>> {
        log::info!("Using fallback segmentation (paragraph mode)");

        if blocks.is_empty() {
            return Ok(vec![]);
        }

        // 计算自适应间距阈值：使用中位数行间距 * 2.0
        let threshold = self.calculate_adaptive_threshold(blocks);
        log::debug!("Adaptive paragraph threshold: {:.1}px", threshold);

        // 按大的垂直间距分段
        let mut questions = vec![];
        let mut current_blocks = vec![];
        let mut last_y = 0.0;

        for block in blocks {
            if !current_blocks.is_empty() && block.bbox.y - last_y > threshold {
                // 新段落
                if !current_blocks.is_empty() {
                    questions.push(self.create_question_from_blocks(
                        &current_blocks,
                        questions.len() + 1,
                    ));
                    current_blocks.clear();
                }
            }

            current_blocks.push(block.clone());
            last_y = block.bbox.y + block.bbox.height;
        }

        // 最后一段
        if !current_blocks.is_empty() {
            questions.push(self.create_question_from_blocks(
                &current_blocks,
                questions.len() + 1,
            ));
        }

        log::info!("Fallback segmentation created {} questions", questions.len());
        Ok(questions)
    }

    /// 计算自适应段落间距阈值
    ///
    /// 使用所有 blocks 之间的行间距中位数 * 2.0 作为"显著间隙"阈值
    fn calculate_adaptive_threshold(&self, blocks: &[TextBlock]) -> f64 {
        if blocks.len() < 2 {
            return 50.0; // 不够数据时回退到默认值
        }

        // 按 y 排序后计算相邻 block 间的间距
        let mut sorted: Vec<&TextBlock> = blocks.iter().collect();
        sorted.sort_by(|a, b| {
            a.bbox
                .y
                .partial_cmp(&b.bbox.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut gaps: Vec<f64> = Vec::with_capacity(sorted.len() - 1);
        for i in 1..sorted.len() {
            let gap = sorted[i].bbox.y - (sorted[i - 1].bbox.y + sorted[i - 1].bbox.height);
            if gap > 0.0 {
                gaps.push(gap);
            }
        }

        if gaps.is_empty() {
            return 50.0;
        }

        // 计算中位数
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if gaps.len() % 2 == 0 {
            (gaps[gaps.len() / 2 - 1] + gaps[gaps.len() / 2]) / 2.0
        } else {
            gaps[gaps.len() / 2]
        };

        // 显著间隙 = 2x 正常行间距，但不低于 20px
        (median * 2.0).max(20.0)
    }

    /// 从 blocks 创建题目
    ///
    /// # Panics
    /// 不会 panic — 如果 blocks 为空，返回零大小框
    fn create_question_from_blocks(&self, blocks: &[TextBlock], index: usize) -> QuestionBox {
        let bbox = match blocks.first() {
            Some(first) => {
                let mut bbox = first.bbox;
                for block in blocks.iter().skip(1) {
                    bbox = bbox.union(&block.bbox);
                }
                bbox
            }
            None => Rect::new(0.0, 0.0, 0.0, 0.0),
        };

        QuestionBox {
            page_index: 0,
            question_id: index.to_string(),
            bounding_box: bbox,
            title_anchor_box: None,
            confidence: 0.5, // 回退模式置信度较低
            recognized_title_text: None,
            block_ids: blocks.iter().map(|b| b.id).collect(),
            debug_info: Some(QuestionDebugInfo {
                num_blocks: blocks.len(),
                detection_method: "fallback_paragraph".to_string(),
                has_options: false,
                has_image: false,
            }),
        }
    }
}

impl Default for QuestionSegmenter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PatternType;

    #[test]
    fn test_segmentation() {
        let segmenter = QuestionSegmenter::new();

        // 创建模拟数据
        let anchors = vec![
            QuestionAnchor {
                question_id: "1".to_string(),
                bbox: Rect::new(50.0, 100.0, 30.0, 20.0),
                confidence: 0.95,
                text: "1.".to_string(),
                pattern_type: PatternType::Numbered,
            },
            QuestionAnchor {
                question_id: "2".to_string(),
                bbox: Rect::new(50.0, 300.0, 30.0, 20.0),
                confidence: 0.95,
                text: "2.".to_string(),
                pattern_type: PatternType::Numbered,
            },
        ];

        let blocks = vec![
            TextBlock {
                id: 0,
                bbox: Rect::new(100.0, 105.0, 400.0, 20.0),
                text: Some("This is question 1 text".to_string()),
                confidence: 0.9,
                block_type: crate::types::BlockType::Text,
            },
            TextBlock {
                id: 1,
                bbox: Rect::new(100.0, 130.0, 400.0, 20.0),
                text: Some("More text for question 1".to_string()),
                confidence: 0.9,
                block_type: crate::types::BlockType::Text,
            },
            TextBlock {
                id: 2,
                bbox: Rect::new(100.0, 305.0, 400.0, 20.0),
                text: Some("This is question 2 text".to_string()),
                confidence: 0.9,
                block_type: crate::types::BlockType::Text,
            },
        ];

        let questions = segmenter.segment(&blocks, &anchors, true).unwrap();

        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].question_id, "1");
        assert_eq!(questions[1].question_id, "2");
        assert_eq!(questions[0].block_ids.len(), 2);
        assert_eq!(questions[1].block_ids.len(), 1);
    }
}
