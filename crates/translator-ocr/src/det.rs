//! Text detection pre/post-processing: PaddleOCR's `DetResizeForTest`, `NormalizeImage` and `DBPostProcess`.

use std::borrow::Cow;

use crate::raster::{Interp, Quad, Rgb8};

/// `DBPostProcess.min_size`: a text region's short side in pixels of the probability map.
const MIN_SIZE: f32 = 3.0;

/// `DBPostProcess` settings.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DbParams {
    /// Probability above which a pixel is text.
    pub thresh: f32,
    /// Mean probability a region needs to count as a box.
    pub box_thresh: f32,
    /// How far a box grows back out from the shrunken region the model predicts.
    pub unclip_ratio: f32,
    pub max_candidates: usize,
}

/// Shrink `image` to a `limit`-px long side, snap both sides to a multiple of 32 (at least 32), and
/// normalize into an NCHW tensor. Returns `(tensor, width, height)`.
pub(crate) fn preprocess(image: &Rgb8, limit: usize) -> (Vec<f32>, usize, usize) {
    // PaddleOCR pads an image under 64 px of width + height, then scales its boxes back by the
    // unpadded size, which squashes them. Stretching to the 32-px minimum keeps them in place.
    let mut image = Cow::Borrowed(image);
    let long = image.width.max(image.height);
    let ratio = if long > limit { limit as f64 / long as f64 } else { 1.0 };
    // Python's `round` sends a tie to the even multiple.
    let snap = |side: usize| (((side as f64 * ratio) as usize as f64 / 32.0).round_ties_even() as usize * 32).max(32);
    let (width, height) = (snap(image.width), snap(image.height));
    if (width, height) != (image.width, image.height) {
        image = Cow::Owned(image.resize(width, height, Interp::Linear));
    }
    // The models read BGR, with the ImageNet statistics applied in that order.
    const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
    const STD: [f32; 3] = [0.229, 0.224, 0.225];
    let mut tensor = vec![0.0; 3 * width * height];
    image.write_chw([2, 1, 0], STD.map(|std| 1.0 / (255.0 * std)), [0, 1, 2].map(|c| -MEAN[c] / STD[c]), &mut tensor, width);
    (tensor, width, height)
}

/// `cv2.minAreaRect`: the smallest rotated rectangle around a point set.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MinRect {
    center: [f32; 2],
    /// Unit vector along `size[0]`; `size[1]` runs perpendicular to it.
    axis: [f32; 2],
    size: [f32; 2],
}

impl MinRect {
    /// `None` when the points do not span an area.
    pub fn of(points: &[[f32; 2]]) -> Option<Self> {
        // Convex hull (monotone chain), then try a rectangle flush with each hull edge.
        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        sorted.dedup();
        let mut hull: Vec<[f32; 2]> = Vec::with_capacity(sorted.len() + 1);
        for half in 0..2 {
            let start = hull.len();
            for &p in &sorted {
                while hull.len() >= start + 2 {
                    let [a, b] = [hull[hull.len() - 2], hull[hull.len() - 1]];
                    let cross = f64::from(b[0] - a[0]) * f64::from(p[1] - a[1]) - f64::from(b[1] - a[1]) * f64::from(p[0] - a[0]);
                    if cross > 0.0 {
                        break;
                    }
                    hull.pop();
                }
                hull.push(p);
            }
            hull.pop();
            if half == 0 {
                sorted.reverse();
            }
        }
        if hull.len() < 3 {
            return None;
        }

        let mut best: Option<(f32, Self)> = None;
        for (i, &from) in hull.iter().enumerate() {
            let to = hull[(i + 1) % hull.len()];
            let length = (to[0] - from[0]).hypot(to[1] - from[1]);
            let axis = [(to[0] - from[0]) / length, (to[1] - from[1]) / length];
            let (mut along, mut across) = ([f32::MAX, f32::MIN], [f32::MAX, f32::MIN]);
            for p in &hull {
                let d = [p[0] - from[0], p[1] - from[1]];
                let (a, c) = (d[0] * axis[0] + d[1] * axis[1], d[1] * axis[0] - d[0] * axis[1]);
                along = [along[0].min(a), along[1].max(a)];
                across = [across[0].min(c), across[1].max(c)];
            }
            let size = [along[1] - along[0], across[1] - across[0]];
            if best.is_none_or(|(area, _)| size[0] * size[1] < area) {
                let (a, c) = ((along[0] + along[1]) * 0.5, (across[0] + across[1]) * 0.5);
                let center = [from[0] + axis[0] * a - axis[1] * c, from[1] + axis[1] * a + axis[0] * c];
                best = Some((size[0] * size[1], Self { center, axis, size }));
            }
        }
        best.map(|(_, rect)| rect)
    }

    /// `cv2.boxPoints`, ordered as PaddleOCR's `get_mini_boxes` does.
    pub fn corners(&self) -> Quad {
        let [ax, ay] = [self.axis[0] * self.size[0] * 0.5, self.axis[1] * self.size[0] * 0.5];
        let [cx, cy] = [-self.axis[1] * self.size[1] * 0.5, self.axis[0] * self.size[1] * 0.5];
        let [x, y] = self.center;
        let mut points = [
            [x - ax - cx, y - ay - cy],
            [x + ax - cx, y + ay - cy],
            [x + ax + cx, y + ay + cy],
            [x - ax + cx, y - ay + cy],
        ];
        points.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let [a, b, c, d] = points;
        let (top_left, bottom_left) = if b[1] > a[1] { (a, b) } else { (b, a) };
        let (top_right, bottom_right) = if d[1] > c[1] { (c, d) } else { (d, c) };
        [top_left, top_right, bottom_right, bottom_left]
    }

    fn short_side(&self) -> f32 {
        self.size[0].min(self.size[1])
    }
}

/// `box_score_fast`: mean of `pred` over the box, filled as `cv2.fillPoly` fills its corners
/// truncated to whole pixels.
fn box_score(pred: &[f32], width: usize, height: usize, quad: &Quad) -> f32 {
    let bound = |axis: usize, max: usize, pick: fn(f32, f32) -> f32, round: fn(f32) -> f32| {
        round(quad.iter().map(|p| p[axis]).reduce(pick).unwrap_or(0.0)).clamp(0.0, max as f32 - 1.0)
    };
    let (x_min, x_max) = (bound(0, width, f32::min, f32::floor), bound(0, width, f32::max, f32::ceil));
    let (y_min, y_max) = (bound(1, height, f32::min, f32::floor), bound(1, height, f32::max, f32::ceil));
    let points = quad.map(|[x, y]| [(x - x_min).trunc(), (y - y_min).trunc()]);

    let (mut sum, mut count) = (0.0_f64, 0_usize);
    for row in 0..=(y_max - y_min) as usize {
        // Where this row crosses the polygon's edges.
        let y = row as f32;
        let mut span: Option<[f32; 2]> = None;
        let mut reach = |x: f32| span = Some(span.map_or([x, x], |[lo, hi]| [lo.min(x), hi.max(x)]));
        for (i, a) in points.iter().enumerate() {
            let b = points[(i + 1) % 4];
            if a[1] == b[1] {
                if a[1] == y {
                    reach(a[0]);
                    reach(b[0]);
                }
            } else if y >= a[1].min(b[1]) && y <= a[1].max(b[1]) {
                reach(a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]));
            }
        }
        let Some([lo, hi]) = span else { continue };
        let (lo, hi) = (lo.round().max(0.0), hi.round().min(x_max - x_min));
        if lo > hi {
            continue;
        }
        let start = (y_min as usize + row) * width + x_min as usize;
        let cells = &pred[start + lo as usize..=start + hi as usize];
        sum += cells.iter().map(|&v| f64::from(v)).sum::<f64>();
        count += cells.len();
    }
    if count == 0 { 0.0 } else { (sum / count as f64) as f32 }
}

/// `DBPostProcess.boxes_from_bitmap`: text boxes of a `width × height` probability map, scaled
/// to the `src_width × src_height` image it was computed from.
///
/// PaddleOCR traces every contour, holes included; this takes each connected region once, so a
/// hole inside a text region does not add a second box.
pub(crate) fn boxes_from_map(
    pred: &[f32],
    width: usize,
    height: usize,
    src_width: usize,
    src_height: usize,
    params: &DbParams,
) -> Vec<Quad> {
    let mut pending: Vec<bool> = pred.iter().map(|&p| p > params.thresh).collect();
    let (scale_x, scale_y) = (src_width as f32 / width as f32, src_height as f32 / height as f32);
    let mut boxes = Vec::new();
    let mut stack = Vec::new();
    let mut outline = Vec::new();
    let mut regions = 0;
    for start in 0..pending.len() {
        if !pending[start] {
            continue;
        }
        if regions == params.max_candidates {
            break;
        }
        regions += 1;

        // Flood-fill the 8-connected region, keeping the pixels on its outline: they have the
        // same convex hull, hence the same rectangle, as the whole region.
        outline.clear();
        pending[start] = false;
        stack.push(start);
        while let Some(at) = stack.pop() {
            let (x, y) = (at % width, at / width);
            let mut edge = x == 0 || y == 0 || x == width - 1 || y == height - 1;
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (x as isize + dx, y as isize + dy);
                if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                    continue;
                }
                let next = ny as usize * width + nx as usize;
                if pred[next] > params.thresh {
                    if pending[next] {
                        pending[next] = false;
                        stack.push(next);
                    }
                } else if dx == 0 || dy == 0 {
                    edge = true;
                }
            }
            if edge {
                outline.push([x as f32, y as f32]);
            }
        }

        let Some(rect) = MinRect::of(&outline) else { continue };
        if rect.short_side() < MIN_SIZE {
            continue;
        }
        let quad = rect.corners();
        if box_score(pred, width, height, &quad) < params.box_thresh {
            continue;
        }
        // `unclip`: pyclipper offsets the corners, truncated to integers, by `distance` with
        // round joins. The smallest rectangle around that offset of a rectangle is the
        // rectangle itself grown by `distance` on every side, so no polygon clipper is needed.
        let distance = rect.size[0] * rect.size[1] * params.unclip_ratio / (2.0 * (rect.size[0] + rect.size[1]));
        let Some(mut grown) = MinRect::of(&quad.map(|p| p.map(f32::trunc))) else {
            continue;
        };
        grown.size = grown.size.map(|side| side + 2.0 * distance);
        if grown.short_side() < MIN_SIZE + 2.0 {
            continue;
        }
        boxes.push(grown.corners().map(|[x, y]| {
            [
                (x * scale_x).round_ties_even().clamp(0.0, src_width as f32),
                (y * scale_y).round_ties_even().clamp(0.0, src_height as f32),
            ]
        }));
    }
    boxes
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARAMS: DbParams = DbParams {
        thresh: 0.3,
        box_thresh: 0.6,
        unclip_ratio: 1.5,
        max_candidates: 1000,
    };

    /// `width × height` map that is 0.9 inside the given inclusive pixel boxes.
    fn map(width: usize, height: usize, blobs: &[[usize; 4]]) -> Vec<f32> {
        let mut pred = vec![0.0; width * height];
        for &[x0, y0, x1, y1] in blobs {
            for y in y0..=y1 {
                pred[y * width + x0..=y * width + x1].fill(0.9);
            }
        }
        pred
    }

    #[test]
    fn preprocess_snaps_sides_to_32_with_ties_to_even() {
        let image = |width, height| Rgb8 {
            width,
            height,
            data: vec![0; width * height * 3],
        };
        // 720 / 32 = 22.5 rounds to 22, 1080 / 32 = 33.75 to 34.
        let (_, width, height) = preprocess(&image(1280, 720), 4000);
        assert_eq!((width, height), (1280, 704));
        let (tensor, width, height) = preprocess(&image(1920, 1080), 960);
        assert_eq!((width, height, tensor.len()), (960, 544, 3 * 960 * 544));
        let (_, width, height) = preprocess(&image(20, 10), 960);
        assert_eq!((width, height), (32, 32), "tiny images are stretched");
    }

    #[test]
    fn preprocess_normalizes_in_bgr_order() {
        let image = Rgb8 {
            width: 32,
            height: 32,
            data: [255, 0, 0].repeat(32 * 32),
        };
        let (tensor, ..) = preprocess(&image, 960);
        let plane = 32 * 32;
        // Blue and green are 0, red is 255.
        assert!((tensor[0] - (-0.485 / 0.229)).abs() < 1e-5);
        assert!((tensor[plane] - (-0.456 / 0.224)).abs() < 1e-5);
        assert!((tensor[2 * plane] - (1.0 - 0.406) / 0.225).abs() < 1e-5);
    }

    #[test]
    fn min_rect_of_upright_and_tilted_points() {
        let rect = MinRect::of(&[[2.0, 3.0], [12.0, 3.0], [12.0, 8.0], [2.0, 8.0], [7.0, 5.0]]).unwrap();
        assert_eq!(rect.corners(), [[2.0, 3.0], [12.0, 3.0], [12.0, 8.0], [2.0, 8.0]]);
        assert_eq!(rect.short_side(), 5.0);

        // A 10√2 × 2√2 rectangle along the diagonal.
        let rect = MinRect::of(&[[0.0, 2.0], [2.0, 0.0], [12.0, 10.0], [10.0, 12.0]]).unwrap();
        assert!((rect.short_side() - 8.0_f32.sqrt()).abs() < 1e-4);
        for (got, want) in rect.corners().iter().zip([[2.0, 0.0], [12.0, 10.0], [10.0, 12.0], [0.0, 2.0]]) {
            assert!((got[0] - want[0]).abs() < 1e-3 && (got[1] - want[1]).abs() < 1e-3, "{got:?} vs {want:?}");
        }

        assert!(MinRect::of(&[[0.0, 0.0], [5.0, 0.0], [9.0, 0.0]]).is_none(), "a line has no area");
    }

    #[test]
    fn box_grows_back_by_the_unclip_distance() {
        // Region 10..=49 × 20..=29: a 39 × 9 rectangle, distance 39 · 9 · 1.5 / 96 ≈ 5.48.
        let pred = map(64, 64, &[[10, 20, 49, 29]]);
        let boxes = boxes_from_map(&pred, 64, 64, 64, 64, &PARAMS);
        assert_eq!(boxes, [[[5.0, 15.0], [54.0, 15.0], [54.0, 34.0], [5.0, 34.0]]]);
    }

    #[test]
    fn boxes_scale_to_the_source_image_and_stay_inside_it() {
        let pred = map(64, 64, &[[0, 20, 63, 29]]);
        let boxes = boxes_from_map(&pred, 64, 64, 128, 32, &PARAMS);
        assert_eq!(boxes.len(), 1);
        let [top_left, _, bottom_right, _] = boxes[0];
        assert_eq!((top_left[0], bottom_right[0]), (0.0, 128.0), "clamped to the image");
        assert!((top_left[1] - 7.0).abs() <= 1.0 && (bottom_right[1] - 17.0).abs() <= 1.0, "{:?}", boxes[0]);
    }

    #[test]
    fn weak_small_and_surplus_regions_are_dropped() {
        // A faint region scores under `box_thresh`; a 2-px-high one is under `MIN_SIZE`.
        let mut pred = map(64, 64, &[[10, 40, 40, 41]]);
        for y in 10..20 {
            pred[y * 64 + 10..y * 64 + 40].fill(0.4);
        }
        assert!(boxes_from_map(&pred, 64, 64, 64, 64, &PARAMS).is_empty());

        let pred = map(64, 64, &[[5, 5, 30, 14], [5, 30, 30, 39], [5, 50, 30, 59]]);
        assert_eq!(boxes_from_map(&pred, 64, 64, 64, 64, &PARAMS).len(), 3);
        let first_two = DbParams {
            max_candidates: 2,
            ..PARAMS
        };
        assert_eq!(boxes_from_map(&pred, 64, 64, 64, 64, &first_two).len(), 2);
    }

    #[test]
    fn diagonal_neighbors_join_one_region() {
        let mut pred = map(64, 64, &[[10, 10, 30, 19]]);
        // Touches the block's corner only diagonally.
        for y in 20..30 {
            pred[y * 64 + 31..y * 64 + 50].fill(0.9);
        }
        // The rectangle around both halves is half empty, so let a low score through.
        let lenient = DbParams { box_thresh: 0.1, ..PARAMS };
        assert_eq!(boxes_from_map(&pred, 64, 64, 64, 64, &lenient).len(), 1);
    }
}
