//! How a kit collection is scheduled under its foliage host.
//!
//! Geometry (`FrondCollection` / `CheapBallCollection`) is the members + probe.
//! This enum is the presenter: bake one mesh, or instance each kit under the same
//! [`crate::FoliageNode`] LOD parent.

/// How a kit collection is turned into scene content.
///
/// [`Self::Merge`] is the default: one [`scene_ref::MultiSceneMerge`] so a quantized
/// unit collection instances a single mesh. [`Self::Instance`] keeps one posed GLB
/// per member — same host and probe, no unique merged mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CollectionPresent {
	/// Bake members into one [`scene_ref::MultiSceneMerge`].
	#[default]
	Merge,
	/// One posed kit per member, siblings under the node placement.
	Instance,
}
