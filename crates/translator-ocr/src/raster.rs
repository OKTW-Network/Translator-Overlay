//! RGB8 raster operations matching the OpenCV calls in PaddleOCR's pipeline.

/// Packed RGB8 pixels, `width * height * 3` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rgb8 {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

/// Four corners in the order top-left, top-right, bottom-right, bottom-left.
pub(crate) type Quad = [[f32; 2]; 4];

#[derive(Debug, Clone, Copy)]
pub(crate) enum Interp {
    /// `cv2.INTER_LINEAR`
    Linear,
    /// `cv2.INTER_CUBIC`
    Cubic,
}

/// `cv2.INTER_CUBIC` weights for the samples at offsets -1, 0, 1 and 2 around a point `t` past the second.
fn cubic_weights(t: f32) -> [f32; 4] {
    const A: f32 = -0.75;
    let u = 1.0 - t;
    let w0 = ((A * (t + 1.0) - 5.0 * A) * (t + 1.0) + 8.0 * A) * (t + 1.0) - 4.0 * A;
    let w1 = ((A + 2.0) * t - (A + 3.0)) * t * t + 1.0;
    let w2 = ((A + 2.0) * u - (A + 3.0)) * u * u + 1.0;
    [w0, w1, w2, 1.0 - w0 - w1 - w2]
}

/// Source indices and weights of every output coordinate along one axis. `cv2.resize` aligns
/// pixel centers, never widens the kernel when shrinking, and repeats the edge pixels.
fn taps(src: usize, dst: usize, interp: Interp) -> Vec<([usize; 4], [f32; 4])> {
    let scale = src as f32 / dst as f32;
    (0..dst)
        .map(|d| {
            let pos = (d as f32 + 0.5) * scale - 0.5;
            let base = pos.floor();
            let t = pos - base;
            let weights = match interp {
                Interp::Linear => [0.0, 1.0 - t, t, 0.0],
                Interp::Cubic => cubic_weights(t),
            };
            let index = [-1, 0, 1, 2].map(|offset| (base as isize + offset).clamp(0, src as isize - 1) as usize);
            (index, weights)
        })
        .collect()
}

/// Run `work(first row, rows)` over bands of `data` (rows of `stride` values) on all cores.
/// A whole frame is resized for every OCR pass, which one core takes ~10 ms over.
fn par_rows<T: Send>(data: &mut [T], stride: usize, work: impl Fn(usize, &mut [T]) + Sync) {
    let rows = data.len() / stride;
    let threads = std::thread::available_parallelism().map_or(1, usize::from).min(rows.div_ceil(64));
    if threads <= 1 {
        return work(0, data);
    }
    let band = rows.div_ceil(threads);
    std::thread::scope(|scope| {
        for (i, rows) in data.chunks_mut(band * stride).enumerate() {
            let work = &work;
            scope.spawn(move || work(i * band, rows));
        }
    });
}

impl Rgb8 {
    /// `cv2.resize` to `width × height`.
    pub fn resize(&self, width: usize, height: usize, interp: Interp) -> Self {
        let columns = taps(self.width, width, interp);
        let rows = taps(self.height, height, interp);
        let stride = width * 3;
        let mut wide = vec![0.0_f32; self.height * stride];
        par_rows(&mut wide, stride, |first, band| {
            let src = self.data[first * self.width * 3..].chunks_exact(self.width * 3);
            for (src, dst) in src.zip(band.chunks_exact_mut(stride)) {
                for (px, (index, weights)) in dst.as_chunks_mut::<3>().0.iter_mut().zip(&columns) {
                    for (c, value) in px.iter_mut().enumerate() {
                        *value = (0..4).map(|k| f32::from(src[index[k] * 3 + c]) * weights[k]).sum();
                    }
                }
            }
        });
        let mut data = vec![0; height * stride];
        par_rows(&mut data, stride, |first, band| {
            for (dst, (index, weights)) in band.chunks_exact_mut(stride).zip(&rows[first..]) {
                let [a, b, c, d] = index.map(|row| &wide[row * stride..(row + 1) * stride]);
                for (i, value) in dst.iter_mut().enumerate() {
                    // `as u8` saturates, so cubic overshoot clamps.
                    *value = (a[i] * weights[0] + b[i] * weights[1] + c[i] * weights[2] + d[i] * weights[3] + 0.5) as u8;
                }
            }
        });
        Self { width, height, data }
    }

    /// The pixel at `(x, y)`, repeating the edge outside the image.
    fn pixel(&self, x: isize, y: isize) -> &[u8] {
        let x = x.clamp(0, self.width as isize - 1) as usize;
        let y = y.clamp(0, self.height as isize - 1) as usize;
        &self.data[(y * self.width + x) * 3..][..3]
    }

    /// PaddleOCR's `get_rotate_crop_image` for a rectangle: bicubic with repeated edges, and a
    /// crop at least 1.5× taller than wide (vertical text) turned 90° counter-clockwise.
    /// `None` when the rectangle is under a pixel wide or tall.
    pub fn crop_quad(&self, quad: &Quad) -> Option<Self> {
        let side = |a: usize, b: usize| (quad[a][0] - quad[b][0]).hypot(quad[a][1] - quad[b][1]);
        let width = side(0, 1).max(side(2, 3)) as usize;
        let height = side(0, 3).max(side(1, 2)) as usize;
        if width == 0 || height == 0 {
            return None;
        }
        let [ox, oy] = quad[0];
        let right = [(quad[1][0] - ox) / width as f32, (quad[1][1] - oy) / width as f32];
        let down = [(quad[3][0] - ox) / height as f32, (quad[3][1] - oy) / height as f32];
        // An upright box on whole pixels (the usual detection) is a plain copy.
        let upright = right == [1.0, 0.0] && down == [0.0, 1.0] && ox.fract() == 0.0 && oy.fract() == 0.0;
        let mut data = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                let sx = ox + right[0] * x as f32 + down[0] * y as f32;
                let sy = oy + right[1] * x as f32 + down[1] * y as f32;
                let (bx, by) = (sx.floor(), sy.floor());
                if upright {
                    data.extend_from_slice(self.pixel(bx as isize, by as isize));
                    continue;
                }
                let (wx, wy) = (cubic_weights(sx - bx), cubic_weights(sy - by));
                let mut px = [0.0_f32; 3];
                for (j, wy) in wy.iter().enumerate() {
                    for (i, wx) in wx.iter().enumerate() {
                        let src = self.pixel(bx as isize + i as isize - 1, by as isize + j as isize - 1);
                        for (value, &s) in px.iter_mut().zip(src) {
                            *value += f32::from(s) * wx * wy;
                        }
                    }
                }
                data.extend(px.map(|value| (value + 0.5) as u8));
            }
        }
        if height * 2 < width * 3 {
            return Some(Self { width, height, data });
        }
        // `np.rot90`: output row `i`, column `j` reads input row `j`, column `width - 1 - i`.
        let mut turned = Vec::with_capacity(data.len());
        for i in 0..width {
            for j in 0..height {
                turned.extend_from_slice(&data[(j * width + width - 1 - i) * 3..][..3]);
            }
        }
        Some(Self {
            width: height,
            height: width,
            data: turned,
        })
    }

    /// Write `value * scale[c] + shift[c]` into the top-left corner of a CHW tensor whose rows
    /// are `stride` wide. Plane `c` reads source channel `channels[c]`.
    pub fn write_chw(&self, channels: [usize; 3], scale: [f32; 3], shift: [f32; 3], out: &mut [f32], stride: usize) {
        let plane = out.len() / 3;
        for (c, out) in out.chunks_exact_mut(plane).enumerate() {
            for (src, dst) in self.data.chunks_exact(self.width * 3).zip(out.chunks_exact_mut(stride)) {
                for (px, value) in src.as_chunks::<3>().0.iter().zip(dst) {
                    *value = f32::from(px[channels[c]]) * scale[c] + shift[c];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `width × height` image whose red channel is `x + 10 * y`.
    fn ramp(width: usize, height: usize) -> Rgb8 {
        let data = (0..width * height)
            .flat_map(|i| [(i % width + 10 * (i / width)) as u8, 0, 255])
            .collect();
        Rgb8 { width, height, data }
    }

    #[test]
    fn linear_resize_matches_opencv() {
        // cv2.resize(np.array([[0, 100, 200, 250]], np.uint8), (2, 1)) == [[50, 225]]
        let src = Rgb8 {
            width: 4,
            height: 1,
            data: [0, 100, 200, 250].iter().flat_map(|&v| [v; 3]).collect(),
        };
        assert_eq!(src.resize(2, 1, Interp::Linear).data, [50, 50, 50, 225, 225, 225]);
        // Upscaling repeats the edge: cv2.resize(np.array([[0, 100]], np.uint8), (4, 1)) == [[0, 25, 75, 100]]
        let src = Rgb8 {
            width: 2,
            height: 1,
            data: vec![0, 0, 0, 100, 100, 100],
        };
        let up = src.resize(4, 1, Interp::Linear);
        assert_eq!(up.data.as_chunks::<3>().0.iter().map(|px| px[0]).collect::<Vec<_>>(), [0, 25, 75, 100]);
    }

    #[test]
    fn cubic_weights_interpolate_and_sum_to_one() {
        assert_eq!(cubic_weights(0.0), [0.0, 1.0, 0.0, 0.0]);
        // OpenCV's table for t = 0.5.
        let w = cubic_weights(0.5);
        for (got, want) in w.iter().zip([-0.09375, 0.59375, 0.59375, -0.09375]) {
            assert!((got - want).abs() < 1e-6, "{w:?}");
        }
    }

    #[test]
    fn upright_crop_copies_pixels() {
        let crop = ramp(8, 8).crop_quad(&[[2.0, 1.0], [6.0, 1.0], [6.0, 3.0], [2.0, 3.0]]).unwrap();
        assert_eq!((crop.width, crop.height), (4, 2));
        assert_eq!(crop.data.as_chunks::<3>().0.iter().map(|px| px[0]).collect::<Vec<_>>(), [12, 13, 14, 15, 22, 23, 24, 25]);
    }

    #[test]
    fn tall_crop_turns_counter_clockwise() {
        let crop = ramp(8, 8).crop_quad(&[[1.0, 0.0], [3.0, 0.0], [3.0, 4.0], [1.0, 4.0]]).unwrap();
        assert_eq!((crop.width, crop.height), (4, 2));
        // The right column becomes the top row, read top to bottom.
        assert_eq!(crop.data.as_chunks::<3>().0.iter().map(|px| px[0]).collect::<Vec<_>>(), [2, 12, 22, 32, 1, 11, 21, 31]);
    }

    #[test]
    fn tilted_crop_follows_the_quad() {
        // A 4×2 box turned 90° clockwise: its top edge runs down the image.
        let crop = ramp(8, 8).crop_quad(&[[4.0, 1.0], [4.0, 5.0], [2.0, 5.0], [2.0, 1.0]]).unwrap();
        assert_eq!((crop.width, crop.height), (4, 2));
        assert_eq!(crop.data.as_chunks::<3>().0.iter().map(|px| px[0]).collect::<Vec<_>>(), [14, 24, 34, 44, 13, 23, 33, 43]);
        assert!(ramp(8, 8).crop_quad(&[[1.0, 1.0], [1.5, 1.0], [1.5, 4.0], [1.0, 4.0]]).is_none(), "under a pixel wide");
    }

    #[test]
    fn chw_planes_pick_channels_and_leave_padding() {
        let src = Rgb8 {
            width: 1,
            height: 1,
            data: vec![10, 20, 30],
        };
        let mut out = [9.0; 6];
        src.write_chw([2, 1, 0], [1.0, 2.0, 3.0], [0.5; 3], &mut out, 2);
        assert_eq!(out, [30.5, 9.0, 40.5, 9.0, 30.5, 9.0]);
    }
}
