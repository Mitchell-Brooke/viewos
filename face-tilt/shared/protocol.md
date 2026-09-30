# ViewOS face-tilt socket protocol

## Transport

- Unix domain stream socket, `SOCK_STREAM`
- Path: `$XDG_RUNTIME_DIR/viewos/face-tilt.sock`
  (systemd `RuntimeDirectory=viewos`; expands to `/run/user/<uid>/viewos/face-tilt.sock`)
- Mode `0600`, owned by the user running the daemon
- The daemon is the **server**; the KWin effect is the **client**
- The effect reconnects automatically if the daemon restarts

## Framing

One UTF-8 JSON object per line, terminated by `\n`. No embedded newlines.
Maximum line length 4096 bytes; longer lines are discarded.

The daemon only ever writes. It accepts no commands. (An earlier draft
documented a `ping` command; that was never needed and has been removed so
that the socket is strictly one-way.)

## Message: head position

```json
{"v":1,"hx":0.5312,"hy":0.4871,"hz":583.0,"roll":-1.4,"conf":0.94,"t":1759253473123}
```

| Field | Type | Units | Meaning |
|-------|------|-------|---------|
| `v`   | int | — | Protocol version, currently `1` |
| `hx`  | float | fraction of screen width | Head position, `0.0` = left edge, `1.0` = right edge |
| `hy`  | float | fraction of screen height | Head position, `0.0` = top edge, `1.0` = bottom edge |
| `hz`  | float | millimetres | Distance from the screen surface to the head |
| `roll`| float | degrees | In-plane head roll. Unused for the per-window facing calculation; reserved |
| `conf`| float | `0.0`–`1.0` | Detection confidence |
| `t`   | int | ms since Unix epoch | Timestamp of the frame the pose was computed from |

## Why fractions rather than pixels

The effect already knows the geometry of every window and the size of the
screen, both in pixels. By expressing the head position as a *fraction* of the
screen, no calibration data needs to cross the socket and the effect needs no
knowledge of the screen's physical size. That keeps a single source of truth
for calibration (the daemon's config file) and makes the effect resolution
independent.

`hz` is in millimetres because viewing distance genuinely is physical: the
amount by which a window needs to rotate depends on how far away the head is,
not on how many pixels wide the screen is.

## Coordinate conventions

The socket uses **screen** coordinates: origin at the top-left of the primary
output, +X right, +Y **down** — matching KWin's window geometry.

`hz` is positive *towards* the viewer.

Within the effect, the head vector is converted to a maths frame with +Y up so
that a positive pitch means "tilt the top of the window away from the viewer".
This sign flip is done in exactly one place, in
`ViewOSFaceTiltEffect::targetRotation()`, so that the convention change is
auditable.

## Rate and loss

The daemon publishes at a fixed rate (default 20 Hz, configurable). Messages
are not queued per client: a slow client gets the most recent state, not a
backlog. Head pose is a continuous quantity where the newest sample is always
the only one worth having, so dropping stale samples is the correct behaviour
rather than a compromise.

## Loss of tracking

When no face is detected the daemon publishes `conf: 0.0` and keeps the last
known `hx`/`hy`/`hz`. The effect treats `conf` below `minConfidence` as "no
pose" and animates the windows back to neutral, then stops repainting.

## Compatibility

Unknown fields must be ignored by readers, so the protocol can be extended
without breaking older builds. A reader that does not recognise `v` should
refuse to connect rather than guess.
