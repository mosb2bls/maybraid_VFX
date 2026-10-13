//! [`LodHostHelper`]: posed kit content for one LOD level root.
//!
//! Domain nodes still implement [`lod::LodScene`]. World [`crate::ComponentsOnly`]
//! hosts drain that **content** via `scene_with_level` (no nested kit hosts).
//! Nested [`lod::LodScene::host`] is for playgrounds and leftover fine-phase hosts.
//!
//! Contrast with [`crate::partitions::host`], which maps **resolution GLB sets**
//! (high / mid / low — ultra-low when authored) onto a [`lod::scene::LodSceneLevel`].

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use scene_ref::SceneRef;

use crate::assets::AssetPath;

/// Posed kit / GLB content helpers for LOD level-root payloads (not host scaffolding).
pub struct LodHostHelper;

impl LodHostHelper {
	/// Optional GLB under a transform (e.g. omit content at a far band).
	pub fn posed_asset_tier(
		asset: Option<AssetPath>,
		transform: Transform,
	) -> impl Scene + 'static {
		Self::posed_scene_ref_tier(asset.map(AssetPath::scene_ref), transform)
	}

	/// Optional [`SceneRef`] under a transform (mirrored kits pass an explicit ref).
	pub fn posed_scene_ref_tier(
		scene_ref: Option<SceneRef>,
		transform: Transform,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = match scene_ref {
			Some(r) => vec![Box::new(r.scene())],
			None => vec![],
		};
		bsn! {
			template_value(transform)
			Visibility::Inherited
			Children [ {children} ]
		}
	}
}
