//! Keep host motion markers in sync with the shown (or desired) LOD level.
//!
//! Runtime truth is on the **host**: [`AnimateBones`] / [`AnimateEffects`] on body
//! rigs, [`ApplyTerrainPitch`] on character roots. Expensive systems filter with
//! `With<…>`. This system inserts/removes those markers from [`motion_policy`],
//! then clamps mailbox work from the plant's [`IntelligenceLod`]. Visible High
//! plants keep bones when the look frustum covers them, even if thinking LOD
//! is Mid / Far.

use bevy::ecs::query::Has;
use bevy::prelude::*;
use intelligence_lod::{look_applies, IntelligenceFocus, IntelligenceLod, IntelligenceLookFrame};
use lod::{LodLevelRoot, LodLevelRoots, LodSceneHost, LodSceneLevel};

use crate::clip::AnimRefRoot;
use crate::markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch};
use crate::pitch::TerrainPitch;
use crate::plant::{plant_lod, plant_lod_entity};
use crate::policy::{clamp_intelligence, motion_policy};
use crate::rig::{CharacterRig, CharacterRigRole};
use crate::shown::shown_level_root;

/// Insert/remove host motion markers from the shown band (else desired / High).
///
/// Mid / Far plants drop bones, effects, and pitch unless they are presenting
/// in the look frustum. Missing lod is Near — a local player keeps the mailbox
/// and terrain pitch.
pub fn sync_motion_markers(
	mut commands: Commands,
	bodies: Query<
		(Entity, &CharacterRig, Has<AnimateBones>, Has<AnimateEffects>),
		With<AnimRefRoot>,
	>,
	pitch_hosts: Query<
		(Entity, Has<ApplyTerrainPitch>),
		Or<(With<ApplyTerrainPitch>, With<TerrainPitch>)>,
	>,
	children: Query<&Children>,
	child_of: Query<&ChildOf>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	visibilities: Query<&Visibility>,
	desired: Query<&LodSceneLevel, With<LodSceneHost>>,
	lods: Query<&IntelligenceLod>,
	transforms: Query<&GlobalTransform>,
	look_frame: Res<IntelligenceLookFrame>,
	focus: Res<IntelligenceFocus>,
) {
	for (entity, rig, has_bones, has_effects) in &bodies {
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let policy = clamp_intelligence(
			motion_policy(motion_level(
				entity,
				&children,
				&level_roots_bags,
				&root_keys,
				&visibilities,
				&desired,
			)),
			plant_lod(entity, &child_of, &lods),
			plant_presenting(entity, &child_of, &lods, &transforms, &look_frame, &focus),
		);
		set_marker::<AnimateBones>(&mut commands, entity, policy.bones, has_bones);
		set_marker::<AnimateEffects>(&mut commands, entity, policy.effects, has_effects);
	}

	for (entity, has_pitch) in &pitch_hosts {
		let policy = clamp_intelligence(
			motion_policy(motion_level(
				entity,
				&children,
				&level_roots_bags,
				&root_keys,
				&visibilities,
				&desired,
			)),
			plant_lod(entity, &child_of, &lods),
			plant_presenting(entity, &child_of, &lods, &transforms, &look_frame, &focus),
		);
		set_marker::<ApplyTerrainPitch>(&mut commands, entity, policy.pitch, has_pitch);
	}
}

/// Look-frustum presentation grant. No published look means no presentation
/// grant — Mid / Far stay clamped so tests without a viewer keep the old cut.
fn plant_presenting(
	start: Entity,
	child_of: &Query<&ChildOf>,
	lods: &Query<&IntelligenceLod>,
	transforms: &Query<&GlobalTransform>,
	look_frame: &IntelligenceLookFrame,
	focus: &IntelligenceFocus,
) -> bool {
	if look_frame.look.is_none() && focus.samples.is_empty() {
		return false;
	}
	let Some(plant) = plant_lod_entity(start, child_of, lods) else {
		return false;
	};
	let Ok(transform) = transforms.get(plant) else {
		return false;
	};
	look_applies(transform.translation(), look_frame.look, focus)
}

fn motion_level(
	host: Entity,
	children: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
	desired: &Query<&LodSceneLevel, With<LodSceneHost>>,
) -> LodSceneLevel {
	if let Some(root) = shown_level_root(host, children, level_roots_bags, root_keys, visibilities)
	{
		if let Ok(key) = root_keys.get(root) {
			return key.0;
		}
	}
	// Fixed assemblies have no LodSceneHost; default visual High, then clamp via IntelligenceLod.
	desired.get(host).copied().unwrap_or(LodSceneLevel::High)
}

fn set_marker<M: Component + Default>(
	commands: &mut Commands,
	entity: Entity,
	want: bool,
	has: bool,
) {
	match (want, has) {
		(true, false) => {
			commands.entity(entity).insert(M::default());
		}
		(false, true) => {
			commands.entity(entity).remove::<M>();
		}
		_ => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::rig::CharacterRig;
	use intelligence_lod::{IntelligenceBand, IntelligenceLook, LOOK_APPLY_FOV_INSET};

	fn run_sync(world: &mut World) {
		use bevy::ecs::system::RunSystemOnce;
		world.init_resource::<IntelligenceLookFrame>();
		world.init_resource::<IntelligenceFocus>();
		world.run_system_once(sync_motion_markers).expect("sync_motion_markers");
	}

	fn looking_x_apply() -> IntelligenceLook {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		IntelligenceLook::from_perspective_inset(
			&GlobalTransform::from(tf),
			&PerspectiveProjection {
				fov: 75_f32.to_radians(),
				aspect_ratio: 16.0 / 9.0,
				..default()
			},
			75_f32.to_radians(),
			LOOK_APPLY_FOV_INSET,
		)
	}

	fn body_on(world: &mut World, parent: Entity) -> Entity {
		world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, ..default() },
				AnimRefRoot::default(),
				AnimateBones,
				AnimateEffects,
				ChildOf(parent),
			))
			.id()
	}

	#[test]
	fn missing_lod_keeps_high_mailbox() {
		let mut world = World::new();
		let plant = world.spawn_empty().id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_some());
		assert!(world.get::<AnimateEffects>(body).is_some());
	}

	#[test]
	fn far_plant_drops_mailbox() {
		let mut world = World::new();
		let plant = world.spawn(IntelligenceLod { band: IntelligenceBand::Far, skips: 0 }).id();
		let visual = world.spawn(ChildOf(plant)).id();
		let body = body_on(&mut world, visual);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_none());
		assert!(world.get::<AnimateEffects>(body).is_none());
	}

	#[test]
	fn near_plant_keeps_mailbox() {
		let mut world = World::new();
		let plant = world.spawn(IntelligenceLod::missing()).id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_some());
		assert!(world.get::<AnimateEffects>(body).is_some());
	}

	#[test]
	fn missing_lod_keeps_pitch() {
		let mut world = World::new();
		let plant = world.spawn_empty().id();
		let host = world.spawn((ApplyTerrainPitch, ChildOf(plant))).id();
		run_sync(&mut world);
		assert!(world.get::<ApplyTerrainPitch>(host).is_some());
	}

	#[test]
	fn far_plant_drops_pitch() {
		let mut world = World::new();
		let plant = world.spawn(IntelligenceLod { band: IntelligenceBand::Far, skips: 0 }).id();
		let host = world.spawn((ApplyTerrainPitch, ChildOf(plant))).id();
		run_sync(&mut world);
		assert!(world.get::<ApplyTerrainPitch>(host).is_none());
	}

	#[test]
	fn mid_ignore_in_frustum_keeps_high_bones() {
		let mut world = World::new();
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let plant = world
			.spawn((
				IntelligenceLod { band: IntelligenceBand::Mid, skips: 0 },
				GlobalTransform::from_translation(Vec3::new(90.0, 1.0, 0.0)),
			))
			.id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_some());
		assert!(world.get::<AnimateEffects>(body).is_some());
	}

	#[test]
	fn mid_ignore_behind_camera_drops_bones() {
		let mut world = World::new();
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let plant = world
			.spawn((
				IntelligenceLod { band: IntelligenceBand::Mid, skips: 0 },
				GlobalTransform::from_translation(Vec3::new(-90.0, 1.0, 0.0)),
			))
			.id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_none());
		assert!(world.get::<AnimateEffects>(body).is_none());
	}
}
