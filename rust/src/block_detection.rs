//! 文本块检测模块
//!
//! 使用几何分析方法检测文本候选区域，不依赖 OCR 的版面分析

use image::GrayImage;
use crate::geometry::Rect;
use crate::types::TextBlock;

/// Block 检测器
pub struct BlockDetector {
    /// 最小 block 面积（过滤噪点）
    min_area: f64,
    /// 最小 block 宽度
    min_width: f64,
    /// 最小 block 高度
    min_height: f64,
}

impl Default for BlockDetector {
    fn default() -> Self {
        Self {
            min_area: 100.0,
            min_width: 10.0,
            min_height: 10.0,
        }
    }
}

impl BlockDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 检测文本块
    pub fn detect(&self, binary_image: &GrayImage) -> anyhow::Result<Vec<TextBlock>> {
        log::info!("Detecting text blocks...");

        // Step 1: 连通域分析
        let components = self.find_connected_components(binary_image)?;
        log::debug!("Found {} connected components", components.len());

        // Step 2: 过滤噪点
        let filtered = self.filter_noise(&components);
        log::debug!("After filtering: {} components", filtered.len());

        // Step 3: 合并近邻 blocks（保守合并，保留行首独立性）
        let merged = self.merge_nearby_blocks(&filtered, binary_image);
        log::debug!("After merging: {} blocks", merged.len());

        // Step 4: 生成 TextBlock 结构
        let text_blocks: Vec<TextBlock> = merged
            .into_iter()
            .enumerate()
            .map(|(id, bbox)| TextBlock {
                id,
                bbox,
                text: None,
                confidence: 1.0,
                block_type: crate::types::BlockType::Text,
            })
            .collect();

        log::info!("Detected {} text blocks", text_blocks.len());
        Ok(text_blocks)
    }

    /// 连通域分析（简化版）
    fn find_connected_components(&self, image: &GrayImage) -> anyhow::Result<Vec<Rect>> {
        let width = image.width() as usize;
        let height = image.height() as usize;

        // 标记数组
        let mut labels = vec![vec![0usize; width]; height];
        let mut next_label = 1usize;
        let mut components: Vec<ComponentInfo> = vec![];

        // 第一次扫描：标记连通域
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x as u32, y as u32)[0];

                // 只处理前景像素（黑色文字）
                if pixel < 128 {
                    let mut neighbors = vec![];

                    // 检查上方和左方的邻居
                    if y > 0 && labels[y - 1][x] > 0 {
                        neighbors.push(labels[y - 1][x]);
                    }
                    if x > 0 && labels[y][x - 1] > 0 {
                        neighbors.push(labels[y][x - 1]);
                    }

                    if neighbors.is_empty() {
                        // 新的连通域
                        labels[y][x] = next_label;
                        components.push(ComponentInfo {
                            label: next_label,
                            min_x: x,
                            max_x: x,
                            min_y: y,
                            max_y: y,
                        });
                        next_label += 1;
                    } else {
                        // 使用最小标签
                        let min_label = *neighbors.iter().min().unwrap();
                        labels[y][x] = min_label;

                        // 更新边界
                        if let Some(comp) = components.iter_mut().find(|c| c.label == min_label) {
                            comp.min_x = comp.min_x.min(x);
                            comp.max_x = comp.max_x.max(x);
                            comp.min_y = comp.min_y.min(y);
                            comp.max_y = comp.max_y.max(y);
                        }
                    }
                }
            }
        }

        // 转换为 Rect
        let rects: Vec<Rect> = components
            .into_iter()
            .map(|comp| {
                Rect::new(
                    comp.min_x as f64,
                    comp.min_y as f64,
                    (comp.max_x - comp.min_x + 1) as f64,
                    (comp.max_y - comp.min_y + 1) as f64,
                )
            })
            .collect();

        Ok(rects)
    }

    /// 过滤噪点
    fn filter_noise(&self, blocks: &[Rect]) -> Vec<Rect> {
        blocks
            .iter()
            .filter(|rect| {
                rect.area() >= self.min_area
                    && rect.width >= self.min_width
                    && rect.height >= self.min_height
            })
            .copied()
            .collect()
    }

    /// 合并近邻 blocks
    ///
    /// 改进策略：只合并水平间距很小（字符间距级别）的 blocks，
    /// 不再跨越大间隙合并，从而保留题号和正文之间的分隔。
    fn merge_nearby_blocks(&self, blocks: &[Rect], _image: &GrayImage) -> Vec<Rect> {
        if blocks.is_empty() {
            return vec![];
        }

        let avg_height = blocks.iter().map(|b| b.height).sum::<f64>() / blocks.len() as f64;

        // 按行分组
        let lines = self.group_into_lines(blocks, avg_height * 0.5);

        // 在每一行内，用较小间距阈值合并（字符间距，而非词间距）
        // 使用 avg_height * 0.2 而不是 0.3，更保守地合并
        let char_gap = avg_height * 0.15;

        let mut merged = vec![];
        for line in lines {
            merged.extend(self.merge_horizontal_neighbors(&line, char_gap));
        }

        merged
    }

    /// 将 blocks 按行分组
    fn group_into_lines(&self, blocks: &[Rect], tolerance: f64) -> Vec<Vec<Rect>> {
        let mut sorted_blocks = blocks.to_vec();
        sorted_blocks.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        let mut lines: Vec<Vec<Rect>> = vec![];
        let mut current_line = vec![];

        for block in sorted_blocks {
            if current_line.is_empty() {
                current_line.push(block);
            } else {
                let last_block = current_line.last().unwrap();
                // 检查是否在同一行
                if (block.y - last_block.y).abs() < tolerance {
                    current_line.push(block);
                } else {
                    lines.push(current_line);
                    current_line = vec![block];
                }
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// 合并水平近邻
    fn merge_horizontal_neighbors(&self, line: &[Rect], max_gap: f64) -> Vec<Rect> {
        if line.is_empty() {
            return vec![];
        }

        let mut sorted_line = line.to_vec();
        sorted_line.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));

        let mut merged = vec![];
        let mut current = sorted_line[0];

        for block in sorted_line.iter().skip(1) {
            let gap = block.x - (current.x + current.width);

            if gap < max_gap {
                // 合并
                current = current.union(block);
            } else {
                merged.push(current);
                current = *block;
            }
        }

        merged.push(current);
        merged
    }

    /// 检测分栏
    #[allow(dead_code)]
    pub fn detect_columns(&self, blocks: &[TextBlock], image_width: u32) -> Vec<ColumnInfo> {
        let mut projection = vec![0usize; image_width as usize];

        for block in blocks {
            let x_start = block.bbox.x as usize;
            let x_end = (block.bbox.x + block.bbox.width) as usize;

            for x in x_start..(x_end.min(image_width as usize)) {
                projection[x] += 1;
            }
        }

        let mut columns: Vec<ColumnInfo> = vec![];
        let mut in_gap = false;
        let mut gap_start = 0;
        let min_gap_width = image_width / 20;

        for (x, &count) in projection.iter().enumerate() {
            if count == 0 {
                if !in_gap {
                    gap_start = x;
                    in_gap = true;
                }
            } else if in_gap {
                let gap_width = x - gap_start;
                if gap_width > min_gap_width as usize {
                    let x_start = columns.last().map(|col| col.x_end).unwrap_or(0.0);
                    columns.push(ColumnInfo {
                        index: columns.len(),
                        x_start,
                        x_end: gap_start as f64,
                    });
                }
                in_gap = false;
            }
        }

        if columns.is_empty() {
            columns.push(ColumnInfo {
                index: 0,
                x_start: 0.0,
                x_end: image_width as f64,
            });
        } else {
            columns.push(ColumnInfo {
                index: columns.len(),
                x_start: columns.last().map(|col| col.x_end).unwrap_or(0.0),
                x_end: image_width as f64,
            });
        }

        log::info!("Detected {} columns", columns.len());
        columns
    }
}

/// 连通域信息
#[derive(Debug, Clone)]
struct ComponentInfo {
    label: usize,
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

/// 分栏信息
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub index: usize,
    pub x_start: f64,
    pub x_end: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_noise() {
        let detector = BlockDetector::default();

        let blocks = vec![
            Rect::new(0.0, 0.0, 5.0, 5.0),    // 太小，应该被过滤
            Rect::new(10.0, 10.0, 50.0, 30.0), // 正常 block
            Rect::new(100.0, 100.0, 100.0, 50.0), // 正常 block
        ];

        let filtered = detector.filter_noise(&blocks);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_group_into_lines() {
        let detector = BlockDetector::default();

        let blocks = vec![
            Rect::new(0.0, 10.0, 50.0, 20.0),   // 第一行
            Rect::new(60.0, 12.0, 50.0, 20.0),  // 第一行
            Rect::new(0.0, 100.0, 50.0, 20.0),  // 第二行
            Rect::new(60.0, 102.0, 50.0, 20.0), // 第二行
        ];

        let lines = detector.group_into_lines(&blocks, 10.0);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].len(), 2);
        assert_eq!(lines[1].len(), 2);
    }
}
