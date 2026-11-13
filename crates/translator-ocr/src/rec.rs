//! Text recognition pre/post-processing: PaddleOCR's `OCRReisizeNormImg` and `CTCLabelDecode`.

use crate::raster::{Interp, Rgb8};

/// Input height of the recognition models.
pub(crate) const HEIGHT: usize = 48;
/// Narrowest input: shorter lines are padded up to it.
const MIN_WIDTH: usize = 320;
/// Widest input: longer lines are squeezed into it.
const MAX_WIDTH: usize = 3200;

/// Resize each line crop to the input height, normalize, and pad on the right to the widest
/// one. Returns the NCHW tensor and its width.
pub(crate) fn batch(crops: &[&Rgb8]) -> (Vec<f32>, usize) {
    let widths: Vec<(usize, usize)> = crops
        .iter()
        .map(|crop| {
            let ratio = crop.width as f64 / crop.height as f64;
            let padded = ((HEIGHT as f64 * ratio.max(MIN_WIDTH as f64 / HEIGHT as f64)) as usize).min(MAX_WIDTH);
            (((HEIGHT as f64 * ratio).ceil() as usize).min(padded), padded)
        })
        .collect();
    let width = widths.iter().map(|&(_, padded)| padded).max().unwrap_or(MIN_WIDTH);
    let mut tensor = vec![0.0; crops.len() * 3 * HEIGHT * width];
    for ((crop, &(resized, _)), out) in crops.iter().zip(&widths).zip(tensor.chunks_exact_mut(3 * HEIGHT * width)) {
        // BGR, scaled to -1..1.
        crop.resize(resized, HEIGHT, Interp::Linear)
            .write_chw([2, 1, 0], [1.0 / 127.5; 3], [-1.0; 3], out, width);
    }
    (tensor, width)
}

/// Greedy CTC decode of one line's `steps × classes.len()` probabilities: the text, and
/// the mean probability of its characters (0 for no text).
pub(crate) fn decode(probs: &[f32], classes: &[String]) -> (String, f32) {
    let mut text = String::new();
    let (mut sum, mut count) = (0.0, 0);
    let mut previous = 0;
    for step in probs.chunks_exact(classes.len()) {
        let (mut best, mut prob) = (0, f32::MIN);
        for (class, &p) in step.iter().enumerate() {
            if p > prob {
                (best, prob) = (class, p);
            }
        }
        // Class 0 is the blank; a class repeated over adjacent steps is one character.
        if best != 0 && best != previous {
            text.push_str(&classes[best]);
            sum += prob;
            count += 1;
        }
        previous = best;
    }
    (text, if count == 0 { 0.0 } else { sum / count as f32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: usize, height: usize, rgb: [u8; 3]) -> Rgb8 {
        Rgb8 {
            width,
            height,
            data: rgb.repeat(width * height),
        }
    }

    #[test]
    fn batch_pads_to_the_widest_line() {
        // 96×24 scales to 192×48 and pads to the 320 minimum; 800×40 to 960×48.
        let (short, long) = (solid(96, 24, [255, 255, 255]), solid(800, 40, [0, 0, 0]));
        let (tensor, width) = batch(&[&short, &long]);
        assert_eq!((width, tensor.len()), (960, 2 * 3 * 48 * 960));
        assert!((tensor[191] - 1.0).abs() < 1e-6, "white");
        assert_eq!(tensor[192], 0.0, "zero padding");
        assert_eq!(tensor[3 * 48 * 960], -1.0, "black");

        assert_eq!(batch(&[&solid(4000, 40, [0, 0, 0])]).1, 3200, "width is capped");
        assert_eq!(batch(&[&short]).1, 320);
    }

    #[test]
    fn batch_reads_channels_as_bgr() {
        let (tensor, width) = batch(&[&solid(320, 48, [255, 0, 0])]);
        let plane = 48 * width;
        assert_eq!([tensor[0], tensor[plane]], [-1.0, -1.0]);
        assert!((tensor[2 * plane] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn decode_collapses_repeats_and_skips_blanks() {
        let classes = ["", "a", "b", " "].map(String::from);
        let step = |class: usize, prob: f32| {
            let mut probs = [0.0; 4];
            probs[class] = prob;
            probs
        };
        // a a _ a b ␠ → "aab "
        let probs = [step(1, 0.9), step(1, 0.5), step(0, 1.0), step(1, 0.7), step(2, 0.8), step(3, 0.6)].concat();
        let (text, score) = decode(&probs, &classes);
        assert_eq!(text, "aab ");
        assert!((score - (0.9 + 0.7 + 0.8 + 0.6) / 4.0).abs() < 1e-6);

        assert_eq!(decode(&step(0, 1.0), &classes), (String::new(), 0.0));
    }
}
