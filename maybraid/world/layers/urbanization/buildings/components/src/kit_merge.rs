//! Pose shared kit GLBs as instances ([`scene_ref::SceneRef`]), not baked merges.
//!
//! Same-kit wall tiles must stay one mesh handle so `write_binned` instances them.
//! [`scene_ref::MultiSceneMerge`] is for vegetation collections whose layouts do not
//! instance across the city.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::LodLazyPending;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use scene_ref::SceneRef;

use crate::lod_host_helper::LodHostHelper;
use crate::parent_confines::{confined_scene, ParentConfines};

#[derive(Clone)]
pub(crate) struct KitPart {
	pub scene: SceneRef,
	pub transform: Transform,
	pub material: Option<MaterialRef>,
	pub confines: ParentConfines,
}

pub(crate) fn scenes_from_kit_parts(parts: Vec<KitPart>) -> Vec<Box<dyn Scene>> {
	parts.into_iter().map(emit_part).collect()
}

fn emit_part(part: KitPart) -> Box<dyn Scene> {
	let scene = posed_kit(part.scene, part.transform, part.material);
	match part.confines {
		ParentConfines::External => scene,
		confines => Box::new(confined_scene(confines, scene)),
	}
}

fn posed_kit(
	scene: SceneRef,
	transform: Transform,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	with_optional_material(LodHostHelper::posed_scene_ref_tier(Some(scene), transform), material)
}

pub(crate) fn with_optional_material(
	scene: impl Scene + 'static,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	match material {
		Some(material) => Box::new((
			bsn! {
				template_value(MaterialRefRoot(material))
				PropagateToDescendants
				LodLazyPending
			},
			scene,
		)),
		None => Box::new(scene),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use scene_ref::SceneRef;

	fn part(path: &str, x: f32) -> KitPart {
		KitPart {
			scene: SceneRef::glb(path),
			transform: Transform::from_xyz(x, 0.0, 0.0),
			material: None,
			confines: ParentConfines::External,
		}
	}

	#[test]
	fn same_kit_instances_stay_separate_posed_scenes() {
		assert_eq!(
			scenes_from_kit_parts(vec![part("wall.glb", 0.0), part("wall.glb", 1.0)]).len(),
			2
		);
	}

	#[test]
	fn internal_confines_wrap_without_baking_into_the_external_run() {
		let internal = KitPart {
			scene: SceneRef::glb("wall.glb"),
			transform: Transform::IDENTITY,
			material: None,
			confines: ParentConfines::internal(bevy::math::Vec3::ZERO, 4.0),
		};
		assert_eq!(scenes_from_kit_parts(vec![part("wall.glb", 0.0), internal]).len(), 2);
	}
}
