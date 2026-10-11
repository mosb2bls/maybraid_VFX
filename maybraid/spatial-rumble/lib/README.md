# Spatial rumble

Reusable camera, ground, and pad rumble from any world-space source. Attacks,
impacts, and footsteps dispatch the same [`SpatialRumble`](src/source.rs)
message; Bray and heavy landing are presets on that type, not a character
contract.

```text
cargo test -p maybraid-spatial-rumble
cargo run -p spatial-rumble-playground
```

## Dispatch

A source names an origin, intensity, duration, envelope, range/falloff, seed,
and [`RumbleChannels`](src/channels.rs). The plugin composes live sources into
one bounded [`SpatialRumbleOutput`](src/compose.rs) each frame.

| Channel | Presentation |
|---|---|
| Camera | Translation + roll on [`SpatialRumbleListener`](src/listener.rs). Shake is a per-frame offset; it does not write look yaw/pitch or locomotion. |
| Ground | [`GroundRumbleSample`](src/channels.rs) — amplitude, frequency, planar direction, origin. Dust, rings, or material motion may consume this. The crate does not displace terrain or spawn hazards. |
| Pad | Rate-limited [`PadRumble`](../../input/src/rumble.rs) from the same composed, bounded output as camera and ground. No gamepad means no vibration; keyboard/mouse play is unchanged. |

## Hosts

Add [`SpatialRumblePlugin`](src/lib.rs) and spawn a listener on the active
camera. Hosts that rewrite `Transform` every frame (follow cameras) set
[`SpatialRumbleListener::host_rewrites_pose`](src/listener.rs) and run
[`SpatialRumbleSystems::Present`](src/compose.rs) after the pose write. Standalone
cameras leave that flag false so the plugin undoes last frame's offset before
applying the next.

Optional [`SpatialRumbleReceiver`](src/listener.rs) samples falloff at a body
pose instead of the camera. Replacing either owner clears applied camera offset
and the composed output.

[`SpatialRumbleSettings`](src/compose.rs) is the intensity and bound control:
master / per-channel gain, max camera offset and roll, max pad motors, max
ground amplitude, a live-source cap, and pad refresh. Listener gain scales
camera presentation once; a [`SpatialRumbleReceiver`](src/listener.rs) scales
compose (falloff / pad / ground) when present.
