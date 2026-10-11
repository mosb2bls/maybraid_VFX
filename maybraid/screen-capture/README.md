# Screen Capture

Isolated Bevy scene capture for inventory and item icons, with a persistent PNG
cache. Parallel to [`scene-ref`](../scene-ref) and
[`material-ref`](../material-ref): a deferred identity, a plugin that fulfills
requests, and a disk-backed handle cache.

## Usage

```rust
use screen_capture::{
    CaptureLighting, CaptureMode, CaptureRequest, CaptureResolution, CaptureRevision,
    CaptureSubject, MeshCaptureSubject, ScreenCapturePlugin,
};

app.add_plugins(ScreenCapturePlugin::new("/tmp/maybraid-icon-cache"));

let subject = MeshCaptureSubject::cuboid([0.8, 0.2, 0.2, 1.0]);
let ticket = client.request(CaptureRequest::from_subject(
    &subject,
    CaptureMode::Shaded,
    CaptureResolution::ICON,
    CaptureLighting::studio(),
    CaptureRevision::CURRENT,
));
let _image = ticket.image;
```

A [`CaptureSubject`] supplies:

- a stable, serializable [`CaptureId`] (visual identity; distinct for
  procedural or customized variants)
- a [`CaptureScene`] (`SceneRef`, a mesh primitive, or an application
  [`CaptureScene::Host`] realized by the caller)
- a preferred [`CaptureView`] (eye, look-at, projection)

Requests specify mode, resolution, lighting, and an explicit capture revision.
Matching in-flight requests are deduplicated. Completion and failure are
reported with [`CaptureCompleted`] / [`CaptureFailed`].

### Modes

- **Shaded** — original materials and shaders, plus the request’s lighting
  preset (studio or a world key/fill pair).
- **Silhouette** — uniform grayscale on a transparent background, with MSAA
  edges. Geometry and standard-material cutouts that affect the outline are
  preserved. Meshes without a `StandardMaterial` get opaque unlit gray.
  The conversion is reapplied through settle and readback so host shader
  systems cannot strip the outline before the GPU copies pixels.

Capture always uses an isolated scene, camera, and render layer. The live
item is never mutated.

### Cache

The cache key covers subject id, mode, resolution, view, lighting
preset/revision, and [`CaptureRevision`]. PNG names may prefix an English
slug (`cylint-wine-snailer_firearm-…`); the slug is not part of the hash.
Hits load the PNG as a Bevy image.
Misses capture, expose the image, then encode and write atomically in the
background. A failed save leaves the in-memory image usable.

Maybraid discovery (first encounter of an item → shaded + silhouette) lives in
the application, not this crate.
