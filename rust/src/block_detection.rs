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

    /// 连通域分析（Union-Find, 8-connectivity）
    fn find_connected_components(&self, image: &GrayImage) -> anyhow::Result<Vec<Rect>> {
        let width = image.width() as usize;
        let height = image.height() as usize;

        // 标记数组
        let mut labels = vec![vec![0usize; width]; height];
        let mut next_label = 1usize;
        let mut uf = UnionFind::new();

        // 第一次扫描：标记连通域，合并等价标签
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x as u32, y as u32)[0];

                // 只处理前景像素（黑色文字）
                if pixel < 128 {
                    let mut neighbor_labels = vec![];

                    // 8-connectivity: 检查上方、左方、左上、右上的邻居
                    if y > 0 {
                        // 上方
                        if labels[y - 1][x] > 0 {
                            neighbor_labels.push(labels[y - 1][x]);
                        }
                        // 左上
                        if x > 0 && labels[y - 1][x - 1] > 0 {
                            neighbor_labels.push(labels[y - 1][x - 1]);
                        }
                        // 右上
                        if x + 1 < width && labels[y - 1][x + 1] > 0 {
                            neighbor_labels.push(labels[y - 1][x + 1]);
                        }
                    }
                    // 左方
                    if x > 0 && labels[y][x - 1] > 0 {
                        neighbor_labels.push(labels[y][x - 1]);
                    }

                    if neighbor_labels.is_empty() {
                        // 新的连通域
                        labels[y][x] = next_label;
                        uf.make_set(next_label);
                        next_label += 1;
                    } else {
                        // 使用最小标签，并 union 所有邻居标签
                        let min_label = *neighbor_labels.iter().min().unwrap();
                        labels[y][x] = min_label;

                        for &lbl in &neighbor_labels {
                            if lbl != min_label {
                                uf.union(min_label, lbl);
                            }
                        }
                    }
                }
            }
        }

        // 第二次扫描：收集每个根标签的 bounding box
        let mut bboxes: std::collections::HashMap<usize, (usize, usize, usize, usize)> =
            std::collections::HashMap::new();

        for y in 0..height {
            for x in 0..width {
                let lbl = labels[y][x];
                if lbl > 0 {
                    let root = uf.find(lbl);
                    let entry = bboxes.entry(root).or_insert((x, x, y, y));
                    entry.0 = entry.0.min(x);
                    entry.1 = entry.1.max(x);
                    entry.2 = entry.2.min(y);
                    entry.3 = entry.3.max(y);
                }
            }
        }

        // 转换为 Rect
        let rects: Vec<Rect> = bboxes
            .values()
            .map(|&(min_x, max_x, min_y, max_y)| {
                Rect::new(
                    min_x as f64,
                    min_y as f64,
                    (max_x - min_x + 1) as f64,
                    (max_y - min_y + 1) as f64,
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

        // 在每一行内，用适当间距阈值合并字符组件
        // 使用 avg_height * 0.3 以合并属于同一词/短语的字符碎片，
        // 但限制最大间距为 15px 以防止过度合并
        let char_gap = (avg_height * 0.3).min(15.0);

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

/// Union-Find（并查集）数据结构，支持路径压缩和按秩合并
struct UnionFind {
    parent: std::collections::HashMap<usize, usize>,
    rank: std::collections::HashMap<usize, usize>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: std::collections::HashMap::new(),
            rank: std::collections::HashMap::new(),
        }
    }

    fn make_set(&mut self, x: usize) {
        self.parent.insert(x, x);
        self.rank.insert(x, 0);
    }

    fn find(&mut self, x: usize) -> usize {
        let p = *self.parent.get(&x).unwrap_or(&x);
        if p != x {
            let root = self.find(p);
            self.parent.insert(x, root);
            root
        } else {
            x
        }
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        let rank_a = *self.rank.get(&ra).unwrap_or(&0);
        let rank_b = *self.rank.get(&rb).unwrap_or(&0);
        if rank_a < rank_b {
            self.parent.insert(ra, rb);
        } else if rank_a > rank_b {
            self.parent.insert(rb, ra);
        } else {
            self.parent.insert(rb, ra);
            self.rank.insert(ra, rank_a + 1);
        }
    }
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
