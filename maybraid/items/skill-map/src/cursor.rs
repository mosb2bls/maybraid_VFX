//! Apply [`SkillMapFlick`] throws to the map camera and bead the stroke.

use bevy::prelude::*;

use crate::controller::{skill_map_steer_held, SkillMapFlick};
use crate::map::{render_layer, SkillMapId};
use crate::tile_material::SkillMapTileAssets;
use crate::user::{SkillMapHeld, SkillMapMember, SkillMapSteerLock, SkillMapUser};
use crate::viewport::{map_view_extent, SkillMapViewportCamera};
use crate::SkillMapEnabled;
use maybraid_character_controller::{OrdinaryLocomotion, RoutedActorControl};

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
	routed: Option<Res<RoutedActorControl>>,
	mut users: Query<(Entity, &mut SkillMapHeld, Option<&OrdinaryLocomotion>), With<SkillMapUser>>,
) {
	for (entity, mut slot, ordinary) in &mut users {
		let ordinary = ordinary.is_none_or(|flag| flag.0);
		slot.0 = enabled.0
			&& if ordinary {
				routed.as_deref().is_some_and(|routed| {
					routed.addresses(entity) && routed.payload().is_some_and(skill_map_steer_held)
				})
			} else {
				true
			};
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
	for flick in flicks.read() {
		if flick.throw() == Vec2::ZERO {
			continue;
		}
		for (map, member, mut transform, projection) in &mut cameras {
			if !flick.addresses(member.user, member.session) {
				continue;
			}
			let Some((user, lock)) = users.iter().find(|(user, _)| user.maps == member.session)
			else {
				continue;
			};
			if lock.locked() {
				continue;
			}
			let start = transform.translation.xy();
			let delta =
				flick_delta(flick.throw(), view_region(projection), user.settings.flick_scale);
			transform.translation.x += delta.x;
			transform.translation.y += delta.y;
			if let Some(assets) = assets.as_deref() {
				bead_flick(
					&mut commands,
					assets,
					*map,
					*member,
					start,
					start + delta,
					transform.translation.z,
				);
			}
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
	fn addressed_flick_moves_only_its_own_map() {
		use crate::user::{spawn_skill_maps, SkillMapMember};

		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<SkillMapEnabled>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, apply_flicks);
		let user_a = app.world_mut().spawn_empty().id();
		let user_b = app.world_mut().spawn_empty().id();
		let maps_a = spawn_skill_maps(&mut app.world_mut().commands(), user_a);
		let maps_b = spawn_skill_maps(&mut app.world_mut().commands(), user_b);
		app.world_mut().flush();
		let cam_a = app
			.world_mut()
			.spawn((
				SkillMapId(0),
				SkillMapMember { user: user_a, session: maps_a },
				SkillMapViewportCamera,
				Transform::default(),
				Projection::Orthographic(OrthographicProjection::default_2d()),
			))
			.id();
		let cam_b = app
			.world_mut()
			.spawn((
				SkillMapId(1),
				SkillMapMember { user: user_b, session: maps_b },
				SkillMapViewportCamera,
				Transform::default(),
				Projection::Orthographic(OrthographicProjection::default_2d()),
			))
			.id();
		app.world_mut().write_message(SkillMapFlick::new(user_a, Some(maps_a), Vec2::X));
		app.update();
		let a = app.world().get::<Transform>(cam_a).expect("cam a").translation.xy();
		let b = app.world().get::<Transform>(cam_b).expect("cam b").translation.xy();
		assert!(a.x > 1.0, "addressed map should move, got {a}");
		assert_eq!(b, Vec2::ZERO, "unrelated map must stay put");
	}

	#[test]
	fn flick_follows_the_stick_across_a_tenth_of_the_view() {
		let region = Vec2::new(91.2, 91.2);
		assert_eq!(flick_delta(Vec2::X, region, 0.1), Vec2::new(9.12, 0.0));
		assert_eq!(flick_delta(Vec2::Y, region, 0.1), Vec2::new(0.0, 9.12));
	}

	#[test]
	fn held_is_rb_while_the_map_is_on() {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_character_controller::{ActorControlSample, RoutedActorControl};
		use maybraid_controllers_core::ActorTarget;

		use crate::user::spawn_skill_maps;

		let mut world = World::new();
		world.init_resource::<SkillMapEnabled>();
		world.init_resource::<RoutedActorControl>();
		let user = world.spawn_empty().id();
		world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.run_system_once(sync_skill_map_held).expect("held without pad");
		assert!(!world.get::<SkillMapHeld>(user).expect("slot").0);

		world.insert_resource(RoutedActorControl::from_sample(
			ActorTarget::new(user),
			ActorControlSample::default().with_bumpers(false, true),
		));
		world.run_system_once(sync_skill_map_held).expect("held with rb");
		assert!(world.get::<SkillMapHeld>(user).expect("slot").0);
	}

	#[test]
	fn tanz_policy_holds_without_rb() {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_character_controller::{OrdinaryLocomotion, RoutedActorControl};

		use crate::user::spawn_skill_maps;

		let mut world = World::new();
		world.init_resource::<SkillMapEnabled>();
		world.init_resource::<RoutedActorControl>();
		let user = world.spawn_empty().id();
		world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.entity_mut(user).insert(OrdinaryLocomotion(false));
		world.run_system_once(sync_skill_map_held).expect("held in tanz");
		assert!(world.get::<SkillMapHeld>(user).expect("slot").0);
	}
}
