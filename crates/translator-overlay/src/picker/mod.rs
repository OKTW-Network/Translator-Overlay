//! On-target OCR region picker.

mod paint;
mod session;

use translator_core::{NormRect, Rect};

use crate::gfx::draw::SurfaceRect;
pub(crate) use crate::picker::session::PickerEnd;

pub const HANDLE_SIZE: i32 = 8;
pub const EDGE_HIT: i32 = 6;
pub const MIN_PX: i32 = 12;

/// Picker geometry. The picker paints in target client pixels, which are surface pixels.
impl SurfaceRect {
    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        let x = a.0.min(b.0);
        let y = a.1.min(b.1);
        Self::new(x, y, (a.0.max(b.0) - x).max(1), (a.1.max(b.1) - y).max(1))
    }

    pub fn from_norm(n: NormRect, client_w: i32, client_h: i32) -> Self {
        let r = n.to_pixel(client_w.max(0) as u32, client_h.max(0) as u32);
        Self::new(r.x.round() as i32, r.y.round() as i32, r.width.round().max(1.0) as i32, r.height.round().max(1.0) as i32)
    }

    pub fn to_norm(self, client_w: i32, client_h: i32) -> Option<NormRect> {
        if client_w <= 0 || client_h <= 0 {
            return None;
        }
        NormRect::from_pixel(Rect::new(self.x as f32, self.y as f32, self.w as f32, self.h as f32), client_w as u32, client_h as u32)
            .sanitize()
    }

    pub fn contains(self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }

    pub fn clamp_inside(self, client_w: i32, client_h: i32) -> Self {
        let w = self.w.clamp(MIN_PX, client_w.max(MIN_PX));
        let h = self.h.clamp(MIN_PX, client_h.max(MIN_PX));
        let x = self.x.clamp(0, (client_w - w).max(0));
        let y = self.y.clamp(0, (client_h - h).max(0));
        Self::new(x, y, w, h)
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
                let max_left = (right - min_w).max(0);
                left = (left + dx).clamp(0, max_left);
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
                let max_top = (bottom - min_h).max(0);
                top = (top + dy).clamp(0, max_top);
            }
            Handle::E | Handle::W => {}
        }

        Self::new(left, top, (right - left).max(1), (bottom - top).max(1))
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
        orig: SurfaceRect,
    },
    Resize {
        index: usize,
        handle: Handle,
        start: (i32, i32),
        orig: SurfaceRect,
    },
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

    fn pixel_rect(&self, region: NormRect) -> SurfaceRect {
        SurfaceRect::from_norm(region, self.client_w, self.client_h)
    }

    pub fn hit_test(&self, px: i32, py: i32) -> Hit {
        // Top-most region (last drawn) first.
        for (index, region) in self.regions.iter().enumerate().rev() {
            let pr = self.pixel_rect(*region);
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
        let start = (px, py);
        self.drag = Some(match self.hit_test(px, py) {
            Hit::Handle { index, handle } => {
                self.selected = Some(index);
                let orig = self.pixel_rect(self.regions[index]);
                DragKind::Resize {
                    index,
                    handle,
                    start,
                    orig,
                }
            }
            Hit::Body { index } => {
                self.selected = Some(index);
                let orig = self.pixel_rect(self.regions[index]);
                DragKind::Move { index, start, orig }
            }
            Hit::None => {
                self.selected = None;
                DragKind::Create { start, current: start }
            }
        });
    }

    pub fn on_move(&mut self, px: i32, py: i32) -> Hit {
        match self.drag {
            Some(DragKind::Create { start, .. }) => {
                let current = (px.clamp(0, self.client_w), py.clamp(0, self.client_h));
                self.drag = Some(DragKind::Create { start, current });
            }
            Some(DragKind::Move { index, start, orig }) => {
                let moved = SurfaceRect::new(orig.x + px - start.0, orig.y + py - start.1, orig.w, orig.h)
                    .clamp_inside(self.client_w, self.client_h);
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
                let resized = orig.apply_resize(handle, px - start.0, py - start.1, self.client_w, self.client_h);
                if let Some(n) = resized.to_norm(self.client_w, self.client_h) {
                    self.regions[index] = n;
                }
            }
            None => {}
        }
        self.hit_test(px, py)
    }

    /// Ends the drag. Returns whether the regions changed.
    pub fn on_left_up(&mut self) -> bool {
        match self.drag.take() {
            Some(DragKind::Create { start, current }) => {
                let start = (start.0.clamp(0, self.client_w), start.1.clamp(0, self.client_h));
                let current = (current.0.clamp(0, self.client_w), current.1.clamp(0, self.client_h));
                let rect = SurfaceRect::from_points(start, current);
                if rect.w < MIN_PX || rect.h < MIN_PX {
                    return false;
                }
                let Some(n) = rect.to_norm(self.client_w, self.client_h) else {
                    return false;
                };
                self.regions.push(n);
                self.selected = Some(self.regions.len() - 1);
                true
            }
            Some(DragKind::Move { .. } | DragKind::Resize { .. }) => true,
            None => false,
        }
    }

    /// Deletes the region under the cursor. Returns whether the regions changed.
    pub fn on_right_up(&mut self, px: i32, py: i32) -> bool {
        if self.drag.is_some() {
            return false;
        }
        match self.hit_test(px, py) {
            Hit::Body { index } | Hit::Handle { index, .. } => {
                self.regions.remove(index);
                self.selected = None;
                true
            }
            Hit::None => false,
        }
    }

    pub fn rubber_band(&self) -> Option<SurfaceRect> {
        match self.drag {
            Some(DragKind::Create { start, current }) => Some(SurfaceRect::from_points(start, current)),
            _ => None,
        }
    }

    pub fn live_pixel_rects(&self) -> Vec<(SurfaceRect, bool)> {
        self.regions
            .iter()
            .enumerate()
            .map(|(i, r)| (self.pixel_rect(*r), self.selected == Some(i)))
            .collect()
    }
}

fn handle_at(pr: SurfaceRect, px: i32, py: i32) -> Option<Handle> {
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
        assert_eq!(p.rubber_band(), Some(SurfaceRect::from_points((20, 50), (80, 110))));
        assert!(p.on_left_up());
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
        assert!(!p.on_left_up());
        assert!(p.regions.is_empty());
    }

    #[test]
    fn right_click_deletes_under_cursor_only() {
        let a = NormRect::new(0.1, 0.2, 0.2, 0.2);
        let b = NormRect::new(0.6, 0.6, 0.2, 0.2);
        let mut p = picker_with(&[a, b]);
        // Miss both → no-op (does not undo last).
        assert!(!p.on_right_up(10, 180));
        assert_eq!(p.regions.len(), 2);
        let hit = SurfaceRect::from_norm(a, 200, 200);
        assert!(p.on_right_up(hit.x + 4, hit.y + 4));
        assert_eq!(p.regions.len(), 1);
        assert!((p.regions[0].x - b.x).abs() < 1e-5);
    }

    #[test]
    fn resize_east_grows_width() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = SurfaceRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        p.on_move(pr.x + pr.w + 20, pr.y + pr.h / 2);
        let _ = p.on_left_up();
        assert!(p.regions[0].width > n.width + 0.05);
    }

    #[test]
    fn resize_past_client_keeps_opposite_edge() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let pr = SurfaceRect::from_norm(n, 200, 200);

        let mut east = picker_with(&[n]);
        east.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        east.on_move(800, pr.y + pr.h / 2);
        let _ = east.on_left_up();
        let out = SurfaceRect::from_norm(east.regions[0], 200, 200);
        assert_eq!(out.x, pr.x);
        assert_eq!(out.x + out.w, 200);
        assert_eq!(out.y, pr.y);
        assert_eq!(out.h, pr.h);

        let mut west = picker_with(&[n]);
        west.on_left_down(pr.x, pr.y + pr.h / 2);
        west.on_move(-400, pr.y + pr.h / 2);
        let _ = west.on_left_up();
        let out = SurfaceRect::from_norm(west.regions[0], 200, 200);
        assert_eq!(out.x, 0);
        assert_eq!(out.x + out.w, pr.x + pr.w);

        let mut south = picker_with(&[n]);
        south.on_left_down(pr.x + pr.w / 2, pr.y + pr.h);
        south.on_move(pr.x + pr.w / 2, 900);
        let _ = south.on_left_up();
        let out = SurfaceRect::from_norm(south.regions[0], 200, 200);
        assert_eq!(out.y, pr.y);
        assert_eq!(out.y + out.h, 200);
        assert_eq!(out.x, pr.x);
        assert_eq!(out.w, pr.w);
    }

    #[test]
    fn resize_east_stops_at_min_width() {
        let n = NormRect::new(0.2, 0.2, 0.2, 0.2);
        let mut p = picker_with(&[n]);
        let pr = SurfaceRect::from_norm(n, 200, 200);
        p.on_left_down(pr.x + pr.w, pr.y + pr.h / 2);
        p.on_move(pr.x - 80, pr.y + pr.h / 2);
        let _ = p.on_left_up();
        let out = SurfaceRect::from_norm(p.regions[0], 200, 200);
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
        assert!(!p.on_left_up());
        assert!(p.regions.is_empty());
    }

    #[test]
    fn create_works_at_top_of_client() {
        let mut p = picker_with(&[]);
        p.on_left_down(10, 4);
        p.on_move(80, 40);
        assert!(p.on_left_up());
        assert_eq!(p.regions.len(), 1);
    }
}
