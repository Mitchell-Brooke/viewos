//! Head pose from facial landmarks.
//!
//! YuNet gives five 2D landmarks. Those are matched against a canonical 3D
//! face model with a perspective-n-point solve, which yields a rotation and a
//! translation. Only the translation is of interest downstream: the effect
//! derives its own per-window angles, and a head *orientation* is not the same
//! thing as a head *position* when the viewer is leaning.

use opencv::{
    core::{Mat, Point2f, Point3f, Size, VectorOfPoint2f, VectorOfPoint3f, CV_32F, CV_64F},
    calib3d::{self, SolvePnPMethod},
    prelude::*,
};

use crate::{
    config::{DetectorConfig, GeometryConfig},
    types::{FaceDetection, HeadPose},
};

/// Canonical 3D positions of the five YuNet landmarks, in millimetres, in the
/// same order YuNet emits them: right eye, left eye, nose tip, right mouth
/// corner, left mouth corner.
///
/// The x signs are negative for the landmark YuNet calls "right" because
/// "right" is the subject's right, which appears on the left of an
/// unmirrored camera image. Getting this backwards produces a head position
/// that is mirrored about the screen centre, which is a genuinely confusing
/// failure to debug from the outside.
///
/// The interocular distance of this model is 66 mm, close to an adult mean, so
/// the absolute scale of the solved translation depends on it.
const MODEL_LANDMARKS: [[f32; 3]; 5] = [
    [-33.0, -13.0, -8.0],  // right eye outer corner
    [33.0, -13.0, -8.0],   // left eye outer corner
    [0.0, 3.0, 24.0],      // nose tip
    [-26.0, 32.0, -4.0],   // right mouth corner
    [26.0, 32.0, -4.0],    // left mouth corner
];

/// Solves head position from detections.
pub struct HeadPoseSolver {
    /// Intrinsic camera matrix, derived from the configured field of view.
    camera_matrix: Mat,
    /// Lens distortion. Zero: webcams are cheap and these assumptions hold
    /// well enough at the distances involved.
    dist_coeffs: Mat,
    /// The frame size the intrinsics were built for.
    frame_size: Size,
    model_points: VectorOfPoint3f,
    geometry: GeometryConfig,
}

impl HeadPoseSolver {
    /// Build a solver for frames of `frame_size`.
    ///
    /// # Calibration
    ///
    /// Webcams do not report their intrinsics, so they are derived from the
    /// horizontal field of view, which the user configures:
    ///
    /// ```text
    /// fx = (frame_width / 2) / tan(hfov / 2)
    /// fy = fx                      (square pixels)
    /// cx = frame_width / 2
    /// cy = frame_height / 2
    /// ```
    ///
    /// A wrong field of view does not cause a visible error in the *angles* of
    /// a head pose; it causes the solved *distance* to be wrong in a way that
    /// scales with the error. That is why it is the only calibration value
    /// exposed.
    pub fn new(
        detector: &DetectorConfig,
        geometry: &GeometryConfig,
        frame_size: Size,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            frame_size.width > 0 && frame_size.height > 0,
            "camera reported a zero-sized frame"
        );

        let hfov = f32::from(detector.horizontal_fov_deg).to_radians();
        let fx = (frame_size.width as f32 / 2.0) / (hfov / 2.0).tan();
        let fy = fx;

        let camera_matrix = Mat::from_slice(&[
            f64::from(fx),
            0.0,
            f64::from(frame_size.width as f32 / 2.0),
            0.0,
            f64::from(fy),
            f64::from(frame_size.height as f32 / 2.0),
            0.0,
            0.0,
            1.0,
        ])?;

        let dist_coeffs = Mat::zeros(4, 1, CV_64F)?;

        let model_points: VectorOfPoint3f = MODEL_LANDMARKS
            .iter()
            .map(|p| Point3f::new(p[0], p[1], p[2]))
            .collect();

        Ok(Self {
            camera_matrix,
            dist_coeffs,
            frame_size,
            model_points,
            geometry: geometry.clone(),
        })
    }

    /// The frame size the intrinsics were built for.
    pub fn frame_size(&self) -> Size {
        self.frame_size
    }

    /// Solve for the head position behind `face`.
    ///
    /// Returns `None` when the solve is degenerate. That happens routinely
    /// rather than exceptionally: a face at a steep angle, motion blur, or a
    /// partial occlusion all make the five points nearly coplanar in view and
    /// the solution flies off to a nonsense distance. Callers should treat
    /// `None` as "no new information" and keep the previous pose.
    pub fn solve(&self, face: &FaceDetection) -> Option<HeadPose> {
        let image_points: VectorOfPoint2f = face.landmarks.iter().copied().collect();

        let mut rvec = Mat::default();
        let mut tvec = Mat::default();

        // SOLVEPNP_ITERATIVE needs at least four points and five are
        // available. It does not need an initial guess, which SQPNP and
        // EPNP would otherwise be preferred for; with five points the
        // iterative refinement is both reliable and fast enough at 20 Hz.
        if calib3d::solve_pnp(
            &self.model_points,
            &image_points,
            &self.camera_matrix,
            &self.dist_coeffs,
            &mut rvec,
            &mut tvec,
            false,
            SolvePnPMethod::SOLVEPNP_ITERATIVE.into(),
        )
        .is_err()
        {
            return None;
        }

        // solve_pnp writes a 3x1 CV_64F column vector.
        if tvec.rows() != 3 || tvec.cols() != 1 {
            return None;
        }
        // SAFETY: the element type and shape were just checked, and
        // `at_2d` bounds-checks independently, returning an error rather than
        // reading out of range.
        let translation = [
            unsafe { *tvec.at_2d::<f64>(0, 0).ok()? } as f32,
            unsafe { *tvec.at_2d::<f64>(1, 0).ok()? } as f32,
            unsafe { *tvec.at_2d::<f64>(2, 0).ok()? } as f32,
        ];

        if translation.iter().any(|v| !v.is_finite()) {
            return None;
        }

        // tvec is in camera coordinates: +X right, +Y **down**, +Z away from
        // the camera into the scene. Convert to screen coordinates: +X right,
        // +Y down (same), and +Z **towards** the viewer, hence negated.
        let mut position_mm = [
            translation[0],
            translation[1],
            -translation[2] + self.geometry.camera_offset_mm[2],
        ];

        // The camera is offset from the screen centre. The config states that
        // offset in the maths frame (+Y up, +Z towards the viewer), so the Y
        // term flips sign on the way in.
        position_mm[0] += self.geometry.camera_offset_mm[0];
        position_mm[1] -= self.geometry.camera_offset_mm[1];

        // A PnP solution that claims the head is behind the screen, or several
        // metres away, is a failed solve. Rejecting it here is what stops
        // windows from spinning wildly for a frame.
        let distance = position_mm[2];
        if !(geometry.min_viewing_distance_mm()..=geometry.max_viewing_distance_mm).contains(&distance)
        {
            return None;
        }

        Some(HeadPose {
            position_mm,
            roll_deg: 0.0,
            confidence: face.confidence,
        })
    }

    /// Whether a face detection has enough spread in its landmarks to be worth
    /// solving. Cheaper than a full solve and rejects some bad detections.
    pub fn landmarks_are_plausible(face: &FaceDetection) -> bool {
        let eye_distance = (face.landmarks[0].x - face.landmarks[1].x).abs();
        let face_height = (face.landmarks[3].y - face.landmarks[0].y).abs();
        eye_distance > 1.0 && face_height > 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::Rect;

    fn solver() -> HeadPoseSolver {
        HeadPoseSolver::new(
            &DetectorConfig::default(),
            &GeometryConfig::default(),
            Size::new(640, 480),
        )
        .expect("solver builds")
    }

    #[test]
    fn intrinsics_are_symmetric_and_centred() {
        let s = solver();
        let m = s.camera_matrix;
        // SAFETY: 3x3 CV_64F, built two lines earlier.
        let at = |r: i32, c: i32| unsafe { *m.at_2d::<f64>(r, c).unwrap() };
        assert_eq!(at(0, 0), at(1, 1), "square pixels assumed");
        assert_eq!(at(0, 2), 320.0);
        assert_eq!(at(1, 2), 240.0);
        assert_eq!(at(2, 2), 1.0);
        // 60 degrees of horizontal field of view over 640 px.
        assert!(
            (at(0, 0) - 554.256).abs() < 1.0,
            "fx was {}, expected about 554.256",
            at(0, 0)
        );
    }

    #[test]
    fn wider_fov_gives_shorter_focal_length() {
        let narrow = HeadPoseSolver::new(
            &DetectorConfig::default(),
            &GeometryConfig {
                horizontal_fov_deg: 30.0,
                ..GeometryConfig::default()
            },
            Size::new(640, 480),
        )
        .unwrap();
        let wide = HeadPoseSolver::new(
            &DetectorConfig::default(),
            &GeometryConfig {
                horizontal_fov_deg: 90.0,
                ..GeometryConfig::default()
            },
            Size::new(640, 480),
        )
        .unwrap();
        let fx = |s: &HeadPoseSolver| unsafe { *s.camera_matrix.at_2d::<f64>(0, 0).unwrap() };
        assert!(fx(&narrow) > fx(&wide));
    }

    #[test]
    fn frontal_face_solves_to_a_plausible_distance() {
        // Project the canonical model through the same camera and feed the
        // result back in. The solver must recover the original distance.
        let s = solver();
        let distance = 600.0_f32;

        let image_points: VectorOfPoint2f = MODEL_LANDMARDS
            .iter()
            .map(|p| {
                // Pinhole projection with the solver's own intrinsics.
                let x = p[0] as f64 / distance as f64 * 554.256 + 320.0;
                let y = p[1] as f64 / distance as f64 * 554.256 + 240.0;
                Point2f::new(x as f32, y as f32)
            })
            .collect();

        let face = FaceDetection {
            bbox: Rect::new(200, 100, 200, 200),
            landmarks: std::array::from_fn(|i| *image_points.get(i).unwrap()),
            confidence: 0.95,
        };

        let pose = s.solve(&face).expect("frontal face solves");
        // The recovered distance should be within about 10% of the truth; the
        // landmark model is approximate so exactness is not expected.
        let recovered = -pose.position_mm[2] - s.geometry.camera_offset_mm[2];
        let error = (recovered - distance).abs() / distance;
        assert!(
            error < 0.10,
            "recovered {recovered} mm for a face at {distance} mm, error {:.1}%",
            error * 100.0
        );
    }

    #[test]
    fn implausible_distance_is_rejected() {
        let s = solver();
        // Landmarks clustered on a single pixel give a singular problem and
        // must not produce a pose.
        let face = FaceDetection {
            bbox: Rect::new(0, 0, 10, 10),
            landmarks: [Point2f::new(320.0, 240.0); 5],
            confidence: 0.99,
        };
        assert!(s.solve(&face).is_none());
    }

    #[test]
    fn degenerate_landmarks_are_rejected_early() {
        let collapsed = FaceDetection {
            bbox: Rect::new(0, 0, 10, 10),
            landmarks: [Point2f::new(5.0, 5.0); 5],
            confidence: 0.9,
        };
        assert!(!HeadPoseSolver::landmarks_are_plausible(&collapsed));

        let sensible = FaceDetection {
            bbox: Rect::new(0, 0, 100, 100),
            landmarks: [
                Point2f::new(30.0, 40.0),
                Point2f::new(70.0, 40.0),
                Point2f::new(50.0, 55.0),
                Point2f::new(38.0, 75.0),
                Point2f::new(62.0, 75.0),
            ],
            confidence: 0.9,
        };
        assert!(HeadPoseSolver::landmarks_are_plausible(&sensible));
    }
}
