//! Screen rectangle in physical pixels. Right and bottom are exclusive.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn from_xywh(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x.saturating_add(width),
            bottom: y.saturating_add(height),
        }
    }

    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }

    pub fn is_empty(self) -> bool {
        self.width() <= 0 || self.height() <= 0
    }

    /// `selected` clipped to this rect, in this rect's client coordinates.
    pub fn client_hole(self, selected: Self) -> Self {
        Self {
            left: selected.left.max(self.left) - self.left,
            top: selected.top.max(self.top) - self.top,
            right: selected.right.min(self.right) - self.left,
            bottom: selected.bottom.min(self.bottom) - self.top,
        }
    }
}
