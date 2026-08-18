//! On-target OCR region picker.

mod paint;
mod session;

use translator_core::{NormRect, Rect};

use crate::gfx::draw::SurfaceRect;
pub(crate) use crate::picker::session::{PickerEnd, is_picker_message};

pub const HANDLE_SIZE: i32 = 8;
pub const EDGE_HIT: i32 = 6;
pub const MIN_PX: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl PixelRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        let x = a.0.min(b.0);
        let y = a.1.min(b.1);
        Self {
            x,
            y,
            w: (a.0.max(b.0) - x).max(1),
            h: (a.1.max(b.1) - y).max(1),
        }
    }

    pub fn from_norm(n: NormRect, client_w: i32, client_h: i32) -> Self {
        let r = n.to_pixel(client_w.max(0) as u32, client_h.max(0) as u32);
        Self {
            x: r.x.round() as i32,
            y: r.y.round() as i32,
            w: r.width.round().max(1.0) as i32,
            h: r.height.round().max(1.0) as i32,
        }
    }

    pub fn to_norm(self, client_w: i32, client_h: i32) -> Option<NormRect> {
        if client_w <= 0 || client_h <= 0 {
            return None;
        }
        NormRect::from_pixel(Rect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32), client_w as u32, client_h as u32)
            .sanitize()
    }

    pub fn to_surface(self) -> SurfaceRect {
        SurfaceRect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }

    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }

    pub fn clamp_inside(self, client_w: i32, client_h: i32) -> Self {
        let w = self.w.clamp(MIN_PX, client_w.max(MIN_PX));
        let h = self.h.clamp(MIN_PX, client_h.max(MIN_PX));
        let x = self.x.clamp(0, (client_w - w).max(0));
        let y = self.y.clamp(0, (client_h - h).max(0));
        Self { x, y, w, h }
    }

    /// Move only the edges of `handle`. The opposite edges stay put, so
    /// overflowing the client does not grow the box the other way.
    fn apply_resize(self, handle: Handle, dx: i32, dy: i32, client_w: i32, client_h: i32) -> Self {
        let min_w = MIN_PX.min(client_w.max(1));
        let min_h = MIN_PX.min(client_h.max(1));
        let mut left = self.x;
        let mut right = self.x + self.w;
        let mut top = self.y;
        let mut bottom = self.y + self.h;

        match handle {
            Handle::E | Handle::NE | Handle::SE => {
                let max_right = client_w.max(left + 1);
                let min_right = (left + min_w).min(max_right);
                right = (right + dx).clamp(min_right, max_right);
            }
            Handle::W | Handle::NW | Handle::SW => {
                let min_left = 0;
                let max_left = (right - min_w).max(min_left);
                left = (left + dx).clamp(min_left, max_left);
            }
            Handle::N | Handle::S => {}
        }
        match handle {
            Handle::S | Handle::SE | Handle::SW => {
                let max_bottom = client_h.max(top + 1);
                let min_bottom = (top + min_h).min(max_bottom);
                bottom = (bottom + dy).clamp(min_bottom, max_bottom);
            }
            Handle::N | Handle::NE | Handle::NW => {
                let min_top = 0;
                let max_top = (bottom - min_h).max(min_top);
                top = (top + dy).clamp(min_top, max_top);
            }
            Handle::E | Handle::W => {}
        }

        Self {
            x: left,
            y: top,
            w: (right - left).max(1),
            h: (bottom - top).max(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Handle {
    pub fn cursor(self) -> PickerCursor {
        match self {
            Self::N | Self::S => PickerCursor::SizeNs,
            Self::E | Self::W => PickerCursor::SizeWe,
            Self::NW | Self::SE => PickerCursor::SizeNwse,
            Self::NE | Self::SW => PickerCursor::SizeNesw,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerCursor {
    Cross,
    SizeAll,
    SizeNs,
    SizeWe,
    SizeNwse,
    SizeNesw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    None,
    Handle { index: usize, handle: Handle },
    Body { index: usize },
}

impl Hit {
    pub fn cursor(self) -> PickerCursor {
        match self {
            Self::Handle { handle, .. } => handle.cursor(),
            Self::Body { .. } => PickerCursor::SizeAll,
            Self::None => PickerCursor::Cross,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Create {
        start: (i32, i32),
        current: (i32, i32),
    },
    Move {
        index: usize,
        start: (i32, i32),
        orig: PixelRect,
    },
    Resize {
        index: usize,
        handle: Handle,
        start: (i32, i32),
        orig: PixelRect,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerAction {
    None,
    RegionsChanged,
}

pub struct RegionPicker {
    pub regions: Vec<NormRect>,
    pub selected: Option<usize>,
    pub client_w: i32,
    pub client_h: i32,
    drag: Option<DragKind>,
}

impl RegionPicker {
    pub fn new(regions: Vec<NormRect>, client_w: i32, client_h: i32) -> Self {
        let regions: Vec<NormRect> = regions.into_iter().filter_map(NormRect::sanitize).collect();
        Self {
            regions,
            selected: None,
            client_w,
            client_h,
            drag: None,
        }
    }

    pub fn set_client_size(&mut self, w: i32, h: i32) {
        self.client_w = w;
        self.client_h = h;
    }

    /// Drop an in-progress drag without committing a new box when the target hides.
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }

    pub fn hit_test(&self, px: i32, py: i32) -> Hit {
        // Top-most region (last drawn) first.
        for (index, region) in self.regions.iter().enumerate().rev() {
            let pr = self
                .live_rect(index)
                .unwrap_or_else(|| PixelRect::from_norm(*region, self.client_w, self.client_h));
            if let Some(handle) = handle_at(pr, px, py) {
                return Hit::Handle { index, handle };
            }
            if pr.contains(px, py) {
                return Hit::Body { index };
            }
        }
        Hit::None
    }

    pub fn on_left_down(&mut self, px: i32, py: i32) {
        match self.hit_test(px, py) {
            Hit::Handle { index, handle } => {
                self.selected = Some(index);
                let orig = PixelRect::from_norm(self.regions[index], self.client_w, self.client_h);
                self.drag = Some(DragKind::Resize {
                    index,
                    handle,
                    start: (px, py),
                    orig,
                });
            }
            Hit::Body { index } => {
                self.selected = Some(index);
                let orig = PixelRect::from_norm(self.regions[index], self.client_w, self.client_h);
                self.drag = Some(DragKind::Move {
                    index,
                    start: (px, py),
                    orig,
                });
            }
            Hit::None => {
                self.selected = None;
                self.drag = Some(DragKind::Create {
                    start: (px, py),
                    current: (px, py),
                });
            }
        }
    }

    pub fn on_move(&mut self, px: i32, py: i32) -> Hit {
        match self.drag {
            Some(DragKind::Create { start, .. }) => {
                let current = (px.clamp(0, self.client_w), py.clamp(0, self.client_h));
                self.drag = Some(DragKind::Create { start, current });
            }
            Some(DragKind::Move { index, start, orig }) => {
                let dx = px - start.0;
                let dy = py - start.1;
                let moved = PixelRect::new(orig.x + dx, orig.y + dy, orig.w, orig.h).clamp_inside(self.client_w, self.client_h);
                if let Some(n) = moved.to_norm(self.client_w, self.client_h) {
                    self.regions[index] = n;
                }
            }
            Some(DragKind::Resize {
                index,
                handle,
                start,
                orig,
            }) => {
                let dx = px - start.0;
                let dy = py - start.1;
                let resized = orig.apply_resize(handle, dx, dy, self.client_w, self.client_h);
                if let Some(n) = resized.to_norm(self.client_w, self.client_h) {
                    self.regions[index] = n;
                }
            }
            None => {}
        }
        self.hit_test(px, py)
    }

    pub fn on_left_up(&mut self, _px: i32, _py: i32) -> PickerAction {
        let drag = self.drag.take();
        match drag {
            Some(DragKind::Create { start, current }) => {
                let start = (start.0.clamp(0, self.client_w), start.1.clamp(0, self.client_h));
                let current = (current.0.clamp(0, self.client_w), current.1.clamp(0, self.client_h));
                let rect = PixelRect::from_points(start, current);
                if rect.w < MIN_PX || rect.h < MIN_PX {
                    return PickerAction::None;
                }
                if let Some(n) = rect.to_norm(self.client_w, self.client_h) {
                    self.regions.push(n);
                    self.selected = Some(self.regions.len() - 1);
                    return PickerAction::RegionsChanged;
                }
                PickerAction::None
            }
            Some(DragKind::Move { .. } | DragKind::Resize { .. }) => PickerAction::RegionsChanged,
            None => PickerAction::None,
        }
    }

    pub fn on_right_up(&mut self, px: i32, py: i32) -> PickerAction {
        if self.drag.is_some() {
            return PickerAction::None;
        }
        match self.hit_test(px, py) {
            Hit::Body { index } | Hit::Handle { index, .. } => {
                if index < self.regions.len() {
                    self.regions.remove(index);
                    self.selected = None;
                    return PickerAction::RegionsChanged;
                }
                PickerAction::None
            }
            _ => PickerAction::None,
        }
    }

    pub fn rubber_band(&self) -> Option<PixelRect> {
        match self.drag {
            Some(DragKind::Create { start, current }) => Some(PixelRect::from_points(start, current)),
            _ => None,
        }
    }

    pub fn live_pixel_rects(&self) -> Vec<(PixelRect, bool)> {
        self.regions
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let pr = PixelRect::from_norm(*r, self.client_w, self.client_h);
                (pr, self.selected == Some(i))
            })
            .collect()
    }

    fn live_rect(&self, index: usize) -> Option<PixelRect> {
        self.regions
            .get(index)
            .map(|r| PixelRect::from_norm(*r, self.client_w, self.client_h))
    }
}

fn handle_at(pr: PixelRect, px: i32, py: i32) -> Option<Handle> {
    let hs = HANDLE_SIZE;
    let near_l = (px - pr.x).abs() <= EDGE_HIT;
    let near_r = (px - (pr.x + pr.w)).abs() <= EDGE_HIT;
    let near_t = (py - pr.y).abs() <= EDGE_HIT;
    let near_b = (py - (pr.y + pr.h)).abs() <= EDGE_HIT;
    let in_x = px >= pr.x - hs && px <= pr.x + pr.w + hs;
    let in_y = py >= pr.y - hs && py <= pr.y + pr.h + hs;
    if !in_x || !in_y {
        return None;
    }
    match (near_t, near_b, near_l, near_r) {
        (true, _, true, _) => Some(Handle::NW),
        (true, _, _, true) => Some(Handle::NE),
        (_, true, true, _) => Some(Handle::SW),
        (_, true, _, true) => Some(Handle::SE),
        (true, _, _, _) => Some(Handle::N),
        (_, true, _, _) => Some(Handle::S),
        (_, _, true, _) => Some(Handle::W),
        (_, _, _, true) => Some(Handle::E),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picker_with(rects: &[NormRect]) -> RegionPicker {
        RegionPicker::new(rects.to_vec(), 200, 200)
    }

    #[test]
    fn create_adds_region() {
        let mut p = picker_with(&[]);
        p.on_left_down(20, 50);
        p.on_move(80, 110);
        assert!(matches!(p.on_left_up(80, 110), PickerAction::RegionsChanged));
        assert_eq!(p.regions.len(), 1);
        let r = p.regions[0];
        assert!(r.x > 0.05 && r.x < 0.15);
        assert!(r.width > 0.25);
    }

    #[test]
    fn tiny_create_ignored() {
        let mut p = picker_with(&[]);
        p.on_left_down(20, 50);
        p.on_move(24, 54);
        assert!(matches!(p.on_left_up(24, 54), PickerAction::None));
        assert!(p.regions.is_empty());
    }

    #[test]
    fn right_click_deletes_under_cursor_only() {
        let a = NormRect::new(0.1, 0.2, 0.2, 0.2);
        let b = NormRect::new(0.6, 0.6, 0.2, 0.2);
        let mut p = picker_with(&[a, b]);
        // Miss both → no-op (does not undo last).
        assert!(matches!(p.on_right_up(10, 180), PickerAction::None));
        assert_eq!(p.regions.len(), 2);
        let hit = PixelRect::from_norm(a, 200, 200);
        assert!(matches!(p.on_right_up(hit.x + 4, hit.y + 4), PickerAction::RegionsChanged));
        assert_eq!(p.regions.len(), 1);
        assert!((p.regions[0].x - b.x).abs() < 1e-5);
    }

    #[test]
    fn resize_east_grows_width() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = PixelRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        p.on_move(pr.x + pr.w + 20, pr.y + pr.h / 2);
        let _ = p.on_left_up(pr.x + pr.w + 20, pr.y + pr.h / 2);
        assert!(p.regions[0].width > n.width + 0.05);
    }

    #[test]
    fn resize_east_past_client_keeps_left_edge() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = PixelRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        p.on_move(800, pr.y + pr.h / 2);
        let _ = p.on_left_up(800, pr.y + pr.h / 2);
        let out = PixelRect::from_norm(p.regions[0], 200, 200);
        assert_eq!(out.x, pr.x);
        assert_eq!(out.x + out.w, 200);
        assert_eq!(out.y, pr.y);
        assert_eq!(out.h, pr.h);
    }

    #[test]
    fn resize_west_past_client_keeps_right_edge() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = PixelRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x, pr.y + pr.h / 2);
        p.on_move(-400, pr.y + pr.h / 2);
        let _ = p.on_left_up(-400, pr.y + pr.h / 2);
        let out = PixelRect::from_norm(p.regions[0], 200, 200);
        assert_eq!(out.x, 0);
        assert_eq!(out.x + out.w, pr.x + pr.w);
    }

    #[test]
    fn resize_south_past_client_keeps_top_edge() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = PixelRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w / 2, pr.y + pr.h);
        p.on_move(pr.x + pr.w / 2, 900);
        let _ = p.on_left_up(pr.x + pr.w / 2, 900);
        let out = PixelRect::from_norm(p.regions[0], 200, 200);
        assert_eq!(out.y, pr.y);
        assert_eq!(out.y + out.h, 200);
        assert_eq!(out.x, pr.x);
        assert_eq!(out.w, pr.w);
    }

    #[test]
    fn resize_east_stops_at_min_width() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = PixelRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        p.on_move(pr.x - 80, pr.y + pr.h / 2);
        let _ = p.on_left_up(pr.x - 80, pr.y + pr.h / 2);
        let out = PixelRect::from_norm(p.regions[0], 200, 200);
        assert_eq!(out.x, pr.x);
        assert_eq!(out.w, MIN_PX);
    }

    #[test]
    fn cancel_drag_drops_rubber_band() {
        let mut p = picker_with(&[]);
        p.on_left_down(20, 50);
        p.on_move(80, 110);
        assert!(p.rubber_band().is_some());
        p.cancel_drag();
        assert!(p.rubber_band().is_none());
        assert!(matches!(p.on_left_up(80, 110), PickerAction::None));
        assert!(p.regions.is_empty());
    }

    #[test]
    fn create_works_at_top_of_client() {
        let mut p = picker_with(&[]);
        p.on_left_down(10, 4);
        p.on_move(80, 40);
        assert!(matches!(p.on_left_up(80, 40), PickerAction::RegionsChanged));
        assert_eq!(p.regions.len(), 1);
    }
}
