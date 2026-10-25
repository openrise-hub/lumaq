use lumaq_core::config::AreaRegion;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn from_points(p1: Point, p2: Point) -> Self {
        let x = p1.x.min(p2.x);
        let y = p1.y.min(p2.y);
        let width = (p1.x - p2.x).unsigned_abs();
        let height = (p1.y - p2.y).unsigned_abs();

        Self {
            x,
            y,
            width: width.max(1),
            height: height.max(1),
        }
    }

    pub fn to_area_region(&self) -> AreaRegion {
        AreaRegion {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    pub fn clamp_to_bounds(&self, max_width: u32, max_height: u32) -> Self {
        let x = self.x.clamp(0, max_width.saturating_sub(1) as i32);
        let y = self.y.clamp(0, max_height.saturating_sub(1) as i32);
        let width = self.width.min(max_width.saturating_sub(x as u32));
        let height = self.height.min(max_height.saturating_sub(y as u32));

        Self {
            x,
            y,
            width: width.max(1),
            height: height.max(1),
        }
    }
}
