//! Apply [`SkillMapFlick`] throws to the map camera and bead the stroke.

use bevy::prelude::*;

use crate::controller::{skill_map_steer_held, SkillMapFlick, SkillMapSteerPolicy};
use crate::map::{render_layer, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::user::{SkillMapHeld, SkillMapMember, SkillMapSteerLock, SkillMapUser};
use crate::viewport::{map_view_extent, SkillMapViewportCamera};
use crate::SkillMapEnabled;
use maybraid_input::VirtualPad;

pub const CURSOR_SPEED: f32 = 64.0;
pub const WATER_LOCK_SECS: f32 = 2.0;
/// Fraction of the visible map region a unit-length flick travels.
pub const FLICK_REGION: f32 = 0.1;
const BEAD_SPACING: f32 = 5.5;
const BEAD_LIFE: f32 = 0.55;
const BEAD_START: f32 = 0.42;
const BEAD_END: f32 = 0.14;

#[derive(Component)]
pub struct SkillMapCursor;

#[derive(Component)]
pub(crate) struct FlickBead {
	age: f32,
	max_age: f32,
	start_scale: f32,
}

pub fn sync_skill_map_held(
	enabled: Res<SkillMapEnabled>,
	policy: Option<Res<SkillMapSteerPolicy>>,
	pad: Option<Res<VirtualPad>>,
	mut users: Query<&mut SkillMapHeld, With<SkillMapUser>>,
) {
	let held = enabled.0
		&& match policy.as_deref().copied().unwrap_or_default() {
			SkillMapSteerPolicy::Standard => pad.as_deref().is_some_and(skill_map_steer_held),
			SkillMapSteerPolicy::Tanz => true,
		};
	for mut slot in &mut users {
		slot.0 = held;
	}
}

pub fn tick_steer_lock(
	time: Res<Time>,
	mut locks: Query<&mut SkillMapSteerLock, With<SkillMapUser>>,
) {
	let dt = time.delta_secs();
	for mut lock in &mut locks {
		if lock.remaining > 0.0 {
			lock.remaining = (lock.remaining - dt).max(0.0);
		}
	}
}

pub fn apply_flicks(
	mut commands: Commands,
	assets: Option<Res<SkillMapTileAssets>>,
	enabled: Res<SkillMapEnabled>,
	mut flicks: MessageReader<SkillMapFlick>,
	users: Query<(&SkillMapUser, &SkillMapSteerLock)>,
	mut cameras: Query<
		(&SkillMapId, &SkillMapMember, &mut Transform, &Projection),
		(With<SkillMapViewportCamera>, Without<SkillMapCursor>),
	>,
) {
	if !enabled.0 {
		for _ in flicks.read() {}
		return;
	}
	let Some(assets) = assets else {
		for _ in flicks.read() {}
		return;
	};
	for flick in flicks.read() {
		if flick.0 == Vec2::ZERO {
			continue;
		}
		for (map, member, mut transform, projection) in &mut cameras {
			let Some((user, lock)) = users.iter().find(|(user, _)| user.maps == member.session)
			else {
				continue;
			};
			if lock.locked() {
				continue;
			}
			let start = transform.translation.xy();
			let delta = flick_delta(flick.0, view_region(projection), user.settings.flick_scale);
			transform.translation.x += delta.x;
			transform.translation.y += delta.y;
			bead_flick(
				&mut commands,
				&assets,
				*map,
				*member,
				start,
				start + delta,
				transform.translation.z,
			);
		}
	}
}

fn view_region(projection: &Projection) -> Vec2 {
	if let Projection::Orthographic(ortho) = projection {
		let size = ortho.area.size();
		if size.min_element() > 8.0 {
			return size;
		}
	}
	map_view_extent()
}

/// Stick space → map. A unit throw crosses `scale` of the visible region, same way as the stick.
pub fn flick_delta(stick: Vec2, region: Vec2, scale: f32) -> Vec2 {
	stick * region * scale
}

fn bead_flick(
	commands: &mut Commands,
	assets: &SkillMapTileAssets,
	map: SkillMapId,
	member: SkillMapMember,
	start: Vec2,
	end: Vec2,
	z: f32,
) {
	let span = end - start;
	let length = span.length();
	if length < 0.5 {
		return;
	}
	let dir = span / length;
	let count = ((length / BEAD_SPACING).floor() as u32).saturating_add(1).max(2);
	for i in 0..count {
		let t = i as f32 / (count - 1) as f32;
		let p = start + dir * (length * t);
		commands.spawn((
			Name::new("skill-map-flick-bead"),
			FlickBead { age: 0.0, max_age: BEAD_LIFE, start_scale: BEAD_START },
			map,
			member,
			Mesh2d(assets.bead_mesh.clone()),
			MeshMaterial2d(assets.cursor.clone()),
			Transform::from_xyz(p.x, p.y, z - 0.2).with_scale(Vec3::splat(BEAD_START)),
			render_layer(map),
		));
	}
}

pub fn tick_flick_beads(
	time: Res<Time>,
	mut commands: Commands,
	mut beads: Query<(Entity, &mut FlickBead, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (entity, mut bead, mut transform) in &mut beads {
		bead.age = (bead.age + dt).min(bead.max_age);
		let u = bead.age / bead.max_age;
		let scale = bead.start_scale.lerp(BEAD_END, u);
		transform.scale = Vec3::splat(scale);
		if bead.age >= bead.max_age {
			commands.entity(entity).despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flick_follows_the_stick_across_a_tenth_of_the_view() {
		let region = Vec2::new(91.2, 91.2);
		assert_eq!(flick_delta(Vec2::X, region, 0.1), Vec2::new(9.12, 0.0));
		assert_eq!(flick_delta(Vec2::Y, region, 0.1), Vec2::new(0.0, 9.12));
	}

	#[test]
	fn held_is_rb_while_the_map_is_on() {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_input::{PadButton, VirtualPad};

		use crate::user::spawn_skill_maps;

		let mut world = World::new();
		world.init_resource::<SkillMapEnabled>();
		let user = world.spawn_empty().id();
		world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.run_system_once(sync_skill_map_held).expect("held without pad");
		assert!(!world.get::<SkillMapHeld>(user).expect("slot").0);

		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		world.insert_resource(pad);
		world.run_system_once(sync_skill_map_held).expect("held with rb");
		assert!(world.get::<SkillMapHeld>(user).expect("slot").0);
	}

	#[test]
	fn tanz_policy_holds_without_rb() {
		use bevy::ecs::system::RunSystemOnce;

		use crate::user::spawn_skill_maps;

		let mut world = World::new();
		world.init_resource::<SkillMapEnabled>();
		world.insert_resource(SkillMapSteerPolicy::Tanz);
		let user = world.spawn_empty().id();
		world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.run_system_once(sync_skill_map_held).expect("held in tanz");
		assert!(world.get::<SkillMapHeld>(user).expect("slot").0);
	}
}
