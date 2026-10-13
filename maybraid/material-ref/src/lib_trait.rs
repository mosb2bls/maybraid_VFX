//! [`MaterialLib`]: resolve [`MaterialRef`] across one or more concrete material types.

use bevy::prelude::{Commands, Entity};

use crate::material_ref::MaterialRef;

/// Capability implemented by a [`bevy::ecs::system::SystemParam`] item that can
/// turn a [`MaterialRef`] into an inserted material component (any `M`).
///
/// Parallel to [`lod::LodSceneRegionIndex`]: the trait is the API; the implementor
/// is a `#[derive(SystemParam)]` struct that borrows only the `Assets` / caches it needs.
///
/// Domain libs should claim only the recipe names they own and return `false` from
/// [`Self::try_fulfill`] for everything else. App crates compose those libs in one
/// [`crate::MaterialRefPlugin`] so two fulfills never race the same root.
///
/// ```ignore
/// #[derive(SystemParam)]
/// pub struct VegetationMaterialLib<'w> {
///     leaf: ResMut<'w, Assets<LeafMaterial>>,
///     cache: ResMut<'w, MaterialRefCache<LeafMaterial>>,
/// }
///
/// impl MaterialLib for VegetationMaterialLib<'_> {
///     fn try_fulfill(&mut self, entity: Entity, r: &MaterialRef, commands: &mut Commands) -> bool {
///         // insert MeshMaterial3d::<LeafMaterial>(…) and return true, or false
///     }
/// }
/// ```
pub trait MaterialLib {
	/// Build or reuse a material when this lib owns `material_ref`.
	///
	/// Return `true` when a material was inserted. Return `false` so a composed
	/// parent lib can try the next domain lib (and eventually a Standard fallback).
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool;

	/// Build or reuse a material for `material_ref` and insert it on `entity`.
	///
	/// The default calls [`Self::try_fulfill`] and ignores the claim flag. Top-level
	/// composed libs should override this to try each child, then a fallback.
	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}
