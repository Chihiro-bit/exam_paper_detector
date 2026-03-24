//! 几何类型定义和工具函数

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// 点坐标
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// 计算到另一个点的欧式距离
    pub fn distance_to(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// 尺寸
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// 矩形框（用于表示文本块、题目框等）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// 从两个点创建矩形（左上角和右下角）
    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).abs();
        let height = (p1.y - p2.y).abs();
        Self::new(x, y, width, height)
    }

    /// 获取中心点
    pub fn center(&self) -> Point {
        Point::new(
            self.x + self.width / 2.0,
            self.y + self.height / 2.0,
        )
    }

    /// 获取面积
    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    /// 获取左上角点
    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// 获取右下角点
    pub fn bottom_right(&self) -> Point {
        Point::new(self.x + self.width, self.y + self.height)
    }

    /// 检查是否包含某个点
    pub fn contains_point(&self, point: &Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// 检查是否与另一个矩形相交
    pub fn intersects(&self, other: &Rect) -> bool {
        !(self.x + self.width < other.x
            || other.x + other.width < self.x
            || self.y + self.height < other.y
            || other.y + other.height < self.y)
    }

    /// 计算与另一个矩形的交集
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        Some(Rect::new(x, y, x2 - x, y2 - y))
    }

    /// 计算与另一个矩形的并集（最小外接矩形）
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);

        Rect::new(x, y, x2 - x, y2 - y)
    }

    /// 计算交并比（IoU - Intersection over Union）
    pub fn iou(&self, other: &Rect) -> f64 {
        if let Some(inter) = self.intersection(other) {
            let inter_area = inter.area();
            let union_area = self.area() + other.area() - inter_area;
            if union_area > 0.0 {
                inter_area / union_area
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// 扩展矩形（向四周扩展指定像素）
    pub fn expand(&self, pixels: f64) -> Rect {
        Rect::new(
            self.x - pixels,
            self.y - pixels,
            self.width + 2.0 * pixels,
            self.height + 2.0 * pixels,
        )
    }

    /// 计算到另一个矩形的最短距离
    pub fn distance_to(&self, other: &Rect) -> f64 {
        if self.intersects(other) {
            return 0.0;
        }

        let dx = if self.x + self.width < other.x {
            other.x - (self.x + self.width)
        } else if other.x + other.width < self.x {
            self.x - (other.x + other.width)
        } else {
            0.0
        };

        let dy = if self.y + self.height < other.y {
            other.y - (self.y + self.height)
        } else if other.y + other.height < self.y {
            self.y - (other.y + other.height)
        } else {
            0.0
        };

        (dx * dx + dy * dy).sqrt()
    }

    /// 检查两个矩形是否垂直对齐（有重叠的 x 范围）
    pub fn is_vertically_aligned(&self, other: &Rect) -> bool {
        let x1_min = self.x;
        let x1_max = self.x + self.width;
        let x2_min = other.x;
        let x2_max = other.x + other.width;

        x1_min < x2_max && x2_min < x1_max
    }

    /// 检查两个矩形是否水平对齐（有重叠的 y 范围）
    pub fn is_horizontally_aligned(&self, other: &Rect) -> bool {
        let y1_min = self.y;
        let y1_max = self.y + self.height;
        let y2_min = other.y;
        let y2_max = other.y + other.height;

        y1_min < y2_max && y2_min < y1_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_area() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(rect.area(), 5000.0);
    }

    #[test]
    fn test_rect_center() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let center = rect.center();
        assert_eq!(center.x, 60.0);
        assert_eq!(center.y, 45.0);
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let inter = r1.intersection(&r2).unwrap();
        assert_eq!(inter.x, 50.0);
        assert_eq!(inter.y, 50.0);
        assert_eq!(inter.width, 50.0);
        assert_eq!(inter.height, 50.0);
    }

    #[test]
    fn test_rect_iou() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let iou = r1.iou(&r2);
        // 交集面积：50*50 = 2500
        // 并集面积：10000 + 10000 - 2500 = 17500
        // IoU = 2500 / 17500 ≈ 0.1429
        assert!((iou - 0.1428571).abs() < 0.0001);
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);
        let union = r1.union(&r2);
        assert_eq!(union.x, 0.0);
        assert_eq!(union.y, 0.0);
        assert_eq!(union.width, 150.0);
        assert_eq!(union.height, 150.0);
    }

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert_eq!(p1.distance_to(&p2), 5.0);
    }

    #[test]
    fn test_rect_contains_point() {
        let rect = Rect::new(10.0, 10.0, 100.0, 100.0);
        assert!(rect.contains_point(&Point::new(50.0, 50.0)));
        assert!(!rect.contains_point(&Point::new(5.0, 5.0)));
    }

    #[test]
    fn test_rect_alignment() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 50.0);
        let r2 = Rect::new(50.0, 60.0, 100.0, 50.0);

        assert!(r1.is_vertically_aligned(&r2));
        assert!(!r1.is_horizontally_aligned(&r2));
    }
}
