//! YuNet face detection.
//!
//! YuNet is a small (about 75k parameter) CNN that OpenCV ships a model for in
//! its model zoo. The model is MIT licensed; see `packages/fetch-yunet-model.sh`
//! for the pinned download and `face-tilt/LICENSES.md` for the attribution.

use opencv::{
    core::{Mat, Point2f, Rect, Scalar, Size, CV_32F},
    dnn::{self, Net},
    prelude::*,
};

use crate::{
    config::DetectorConfig,
    types::FaceDetection,
};

/// Number of values YuNet emits per detection: x, y, w, h, score, then five
/// landmarks as x/y pairs.
const VALUES_PER_DETECTION: usize = 15;

/// Column offsets within one detection row.
const COL_BOX_X: usize = 0;
const COL_BOX_Y: usize = 1;
const COL_BOX_W: usize = 2;
const COL_BOX_H: usize = 3;
const COL_SCORE: usize = 4;
const COL_LANDMARK: usize = 5;

/// Mean subtracted by YuNet's reference preprocessing. The model was trained
/// with inputs mapped to `[-1, 1]`, so `(x - 127.5) / 127.5` is required.
///
/// Using a zero mean, which is what the original scaffolding did, does not
/// fail: the network still returns numbers, they are just meaningless, and
/// detection quality collapses. This is the kind of bug that looks like "the
/// camera is bad".
const YUNET_MEAN: Scalar = Scalar::new(127.5, 127.5, 127.5, 0.0);

/// A loaded YuNet network.
pub struct YuNetDetector {
    net: Net,
    input_size: Size,
    confidence_threshold: f32,
    max_candidates: usize,
}

impl YuNetDetector {
    /// Load the model named in `config`.
    ///
    /// The error distinguishes "the file is not there" from "the file is there
    /// but is not a model", because those have very different fixes: the first
    /// is a packaging bug, the second is usually a truncated download.
    pub fn new(config: &DetectorConfig) -> anyhow::Result<Self> {
        let path = std::path::Path::new(&config.model_path);
        anyhow::ensure!(
            path.exists(),
            "YuNet model not found at {}. \
             On Debian this normally means viewos-face-daemon was installed \
             without its model; reinstall the package, or run \
             packages/fetch-yunet-model.sh and set detector.model_path.",
            config.model_path
        );

        let net = dnn::read_net_from_onnx_and_try_to_read(p).map_err(|e| {
            anyhow::anyhow!(
                "could not load the YuNet model at {}: {e}. \
                 A truncated download is the usual cause; re-run \
                 packages/fetch-yunet-model.sh to fetch it again.",
                config.model_path
            )
        })?;

        // Explicitly avoid any accelerated backend. The default backend choice
        // varies by OpenCV build and can pull in CUDA, which is not installed
        // on a machine using the nouveau driver, turning a working detector
        // into a hard failure.
        net.set_preferable_backend(dnn::DNN_BACKEND_OPENCV)?;
        net.set_preferable_target(dnn::DNN_TARGET_CPU)?;

        Ok(Self {
            net,
            input_size: Size::new(320, 320),
            confidence_threshold: config.confidence_threshold,
            max_candidates: config.max_candidates,
        })
    }

    /// Detect faces in a BGR frame.
    ///
    /// Returns detections sorted by descending confidence, with overlapping
    /// boxes suppressed.
    pub fn detect(&mut self, frame: &Mat) -> anyhow::Result<Vec<FaceDetection>> {
        anyhow::ensure!(!frame.empty(), "cannot run detection on an empty frame");

        let blob = self.preprocess(frame)?;
        self.net.set_input(&blob, "", 0.0, 0.0)?;

        let mut output = Mat::default();
        self.net.forward(&mut output, &[])?;

        let rows = output.reshape(1, 0)?;
        let mut faces = Vec::new();

        // Values are read defensively: a truncated or otherwise damaged model
        // can yield a row with fewer columns than expected, and a panic in a
        // face-tracking daemon means every window snaps back to flat.
        let wanted = (self.max_candidates as i32).min(rows.rows());
        for row_index in 0..wanted {
            let Some(row) = self.read_row(&rows, row_index) else {
                break;
            };
            if row.len() < VALUES_PER_DETECTION {
                break;
            }

            let confidence = row[COL_SCORE];
            if !confidence.is_finite() || confidence < self.confidence_threshold {
                continue;
            }

            // The network emits a heatmap of candidate anchors, of which the
            // overwhelming majority are below threshold. Skipping the rest is
            // just an optimisation: the ordering is by score, not by
            // position, so we cannot stop early.
            let Some(face) = row_to_face(&row, frame, self.input_size) else {
                continue;
            };
            faces.push(face);
        }

        faces.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        Ok(suppress_overlaps(faces))
    }

    /// Scale the frame into the network's input tensor.
    fn preprocess(&self, frame: &Mat) -> opencv::Result<Mat> {
        // scale 1.0, then subtract the 127.5 mean, and swap R and B because
        // YuNet was trained on RGB. This matches OpenCV's own YuNet sample.
        dnn::blob_from_image(
            frame,
            1.0,
            self.input_size,
            YUNET_MEAN,
            true,  // swap_rb
            false, // crop
            CV_32F,
        )
    }

    /// Copy one output row into a `Vec`, or `None` if the row is unreadable.
    fn read_row(&self, rows: &Mat, index: i32) -> Option<Vec<f32>> {
        let row = rows.row(index).ok()?;
        let mut values = Vec::with_capacity(VALUES_PER_DETECTION);
        for column in 0..VALUES_PER_DETECTION {
            // SAFETY: the output tensor is CV_32F, and the loop is bounded to
            // one row and to the column count we know the model produces. The
            // bounds are re-checked by `at_2d` itself, which returns an error
            // rather than reading out of range.
            match unsafe { row.at_2d::<f32>(0, column as i32) } {
                Ok(value) if value.is_finite() => values.push(*value),
                _ => return None,
            }
        }
        Some(values)
    }
}

/// Convert one output row into a detection in frame coordinates.
fn row_to_face(row: &[f32], frame: &Mat, input_size: Size) -> Option<FaceDetection> {
    let scale_x = frame.cols() as f32 / input_size.width as f32;
    let scale_y = frame.rows() as f32 / input_size.height as f32;

    let confidence = row[COL_SCORE];
    let x = row[COL_BOX_X];
    let y = row[COL_BOX_Y];
    let w = row[COL_BOX_W];
    let h = row[COL_BOX_H];

    // A zero-area box is not a face; it is a rejected anchor.
    if !(w > 0.0 && h > 0.0) {
        return None;
    }

    let mut landmarks = [Point2f::default(); 5];
    for (index, landmark) in landmarks.iter_mut().enumerate() {
        let lx = row[COL_LANDMARK + index * 2] * scale_x;
        let ly = row[COL_LANDMARK + index * 2 + 1] * scale_y;
        if !lx.is_finite() || !ly.is_finite() {
            return None;
        }
        *landmark = Point2f::new(lx, ly);
    }

    // Clamp to the frame. A webcam often reports boxes that run slightly past
    // the edge, and KWin would try to paint landmarks that are off-frame.
    let scaled = Rect::new(
        (x * scale_x) as i32,
        (y * scale_y) as i32,
        (w * scale_x) as i32,
        (h * scale_y) as i32,
    );
    let bbox = scaled & frame_size_rect(frame);

    if bbox.width <= 0 || bbox.height <= 0 {
        return None;
    }

    Some(FaceDetection {
        bbox,
        landmarks,
        confidence,
    })
}

/// A rectangle covering the whole frame.
fn frame_size_rect(frame: &Mat) -> Rect {
    Rect::new(0, 0, frame.cols(), frame.rows())
}

/// Intersection over union of two rectangles.
fn iou(a: &Rect, b: &Rect) -> f32 {
    let intersection_width = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let intersection_height = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    if intersection_width <= 0 || intersection_height <= 0 {
        return 0.0;
    }
    let intersection = (intersection_width * intersection_height) as f32;
    let union = (a.width * a.height + b.width * b.height) as f32 - intersection;
    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Greedy non-maximum suppression.
///
/// YuNet's output is a dense anchor grid with no built-in suppression, so the
/// same face routinely appears several times. Without this, pose solving
/// against whichever duplicate happened to come first is noticeably noisier.
fn suppress_overlaps(mut faces: Vec<FaceDetection>) -> Vec<FaceDetection> {
    let mut kept: Vec<FaceDetection> = Vec::with_capacity(faces.len());
    for face in faces.drain(..) {
        if kept.iter().any(|k| iou(&k.bbox, &face.bbox) > 0.3) {
            continue;
        }
        kept.push(face);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iou_of_disjoint_rects_is_zero() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(20, 20, 10, 10);
        assert_eq!(iou(&a, &b), 0.0);
    }

    #[test]
    fn iou_of_identical_rects_is_one() {
        let a = Rect::new(5, 5, 10, 10);
        assert_eq!(iou(&a, &a), 1.0);
    }

    #[test]
    fn iou_of_half_overlap() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 0, 10, 10);
        // intersection 50, union 150
        assert!((iou(&a, &b) - 1.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn suppression_drops_duplicates() {
        let face = FaceDetection {
            bbox: Rect::new(10, 10, 40, 40),
            landmarks: [Point2f::default(); 5],
            confidence: 0.9,
        };
        // One pixel apart, so heavily overlapping.
        let near = FaceDetection {
            bbox: Rect::new(11, 11, 40, 40),
            ..face
        };
        let kept = suppress_overlaps(vec![face, near]);
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn suppression_keeps_separate_faces() {
        let a = FaceDetection {
            bbox: Rect::new(0, 0, 40, 40),
            landmarks: [Point2f::default(); 5],
            confidence: 0.9,
        };
        let b = FaceDetection {
            bbox: Rect::new(200, 200, 40, 40),
            landmarks: [Point2f::default(); 5],
            confidence: 0.8,
        };
        assert_eq!(suppress_overlaps(vec![a, b]).len(), 2);
    }

    #[test]
    fn zero_area_box_is_not_a_face() {
        let row = vec![0.0; VALUES_PER_DETECTION];
        // rows() is needed for the frame rect, so exercise a tiny 4x3 frame.
        let frame = Mat::new_rows_cols_with_default(
            4,
            3,
            CV_32F,
        )
        .expect("tiny frame");
        let face = row_to_face(&row, &frame, Size::new(320, 320));
        assert!(face.is_none());
    }
}
