# Tanz user

Tanzer control mode, starter Tanz definitions, and the Laufer / Tanzer charge HUD.

Hold LB+RB for one second to switch between **Laufer** (ordinary locomotion) and
**Tanzer**. A corner plate shows the active mode and a Maybraid-style charge bar
(0 = Laufer, 1 = Tanzer). Releasing the chord before the bar completes drains or
fills back to the current mode's rest at the same rate. Both bumpers must
release before another toggle.

When a Tanz is executed, a top-center callout shows that Tanz's generated
silhouette and display name, then fades over 1.25s. A newer trigger replaces
the previous callout. The callout is display-only.

## Icons

Each Tanz implements [`screen_capture::RepresentativePose`]: the live
[`AnimClip`] plus that clip's `representative_progress()` (salute hold, strafe
glide, dash held lunge). A blanket [`screen_capture::CaptureSubject`] impl
turns that still into an isolated host scene.

[`item-icons`](../icons) realizes the host as a base nude Braidman
(`BraidmanConfig::default_preview()`, no clothing / hair) via the same
fixed assembly the live character uses, scrubs the clip through the
existing HumanoidPose resolver, and captures
[`CaptureMode::Silhouette`] via [`ScreenCapturePlugin`]. First encounter
writes a 128×128 PNG into the shared icon cache; later launches load the
file. Pending captures leave the HUD icon slot hidden so the name still
shows.

PowerDash / PowerStrafe choreography edits move the icon automatically:
`pose_revision` is a fingerprint of the sampled pose at the representative
progress, so the cache key changes when the live clip's still changes.

Manual playground preview for one pose:

```text
/character assemble --species braidman --animation power-dash
/character playback --pause --progress 0.50
/character camera three-quarter
```
