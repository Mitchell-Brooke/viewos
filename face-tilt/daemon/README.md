# viewos-face-daemon

Captures frames from a webcam, finds the viewer's face with a locally executed
[YuNet](https://github.com/opencv/opencv_zoo) model, works out where the head
is relative to the screen, and publishes that position on a Unix socket. The
companion KWin effect, `viewos-kwin-effect`, reads the socket and rotates each
window to face the viewer.

## Privacy

Everything happens on the machine. The daemon has no networking code beyond the
`AF_UNIX` listener, and the packaged systemd unit independently enforces that
with `RestrictAddressFamilies=AF_UNIX` and `PrivateNetwork=yes`. The model
runs through OpenCV's DNN module; nothing is sent anywhere. See `PRIVACY.md`
for the full statement.

## How head position is derived

1. **Detection.** YuNet runs on each frame and returns five landmarks per face
   plus a bounding box and a confidence. Overlapping detections are suppressed
   with non-maximum suppression, because the network's output is a dense anchor
   grid with no suppression of its own.
2. **Pose.** The five landmarks are matched against a canonical 3D face model
   with `solvePnP`, which yields a rotation and a translation. The translation
   is the useful part: where a head is *pointing* and where it *is* are
   different quantities, and only the latter determines how far a window has to
   rotate.
3. **Conversion.** The translation is expressed in millimetres, then published
   as a fraction of the screen. Fractions mean the effect needs no knowledge of
   the panel's physical size, so there is exactly one place calibration is
   configured.

The only value that needs to be right is `detector.horizontal_fov_deg`, because
webcams do not report their intrinsics. A wrong field of view does not break
detection or pose; it scales the recovered distance, which shows up as windows
tilting as though you were further away than you are.

## Building

Needs a Rust toolchain, `cmake`, `pkg-config` and Debian's `libopencv-dev`.

```
cargo build --release
```

The `opencv` crate will otherwise build a vendored copy of OpenCV, which takes
the best part of an hour. `packages/build-face-daemon.sh` sets the `OPENCV_*`
environment variables that make it use the system copy instead, and is the
supported way to build the Debian package.

## Layout

| File | Purpose |
|------|---------|
| `src/main.rs` | Argument handling, capture loop, shutdown |
| `src/config.rs` | Configuration file schema and validation |
| `src/detector.rs` | YuNet loading, preprocessing, non-maximum suppression |
| `src/pose.rs` | `solvePnP` and camera intrinsics |
| `src/socket.rs` | The publication socket |
| `src/types.rs` | Wire types, shared with the KWin effect |

## Tests

```
cargo test
```

The tests cover the parts that can be checked without a camera: configuration
validation, non-maximum suppression, intrinsics derivation, a synthetic
frontal-face pose solve, and the wire format. The camera path itself is only
covered by running the daemon.
