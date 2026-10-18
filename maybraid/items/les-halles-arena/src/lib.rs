//! Fixed-envelope Les Halles arena for the firing-range free-for-all.
//!
//! The footprint is 36×10×36 m. The firing range mounts seed [`PLAYGROUND_SEED`].

mod pad;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use building_components::{building_bounds, spawn_building_components};
use building_physics::{spawn_building_walk_colliders, BUILDING_FRICTION};
use buildings::{Confines, Fit, FitError, Openings};
use movement_intelligence_richmond::{
	circulation_from_stairwell, circulation_from_storey, CirculationStairwell,
};
use procedural_common::NoiseParams;
use urbanization_developments::MixedUseLesHallesDevelopment;

pub use pad::{spawn_pad, ArenaPad};

/// Footprint large enough for a monotower (`≥ 28 m`) and two storeys (`height ≥ 10 m`).
pub const ARENA_EXTENT: Vec3 = Vec3::new(36.0, 10.0, 36.0);
/// Reproducible firing-range bench.
pub const PLAYGROUND_SEED: i32 = 1337;

/// Marks every pad and building host so a session can despawn the set.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TrainingArena;

/// Capsule spawn poses next to a ground-floor stair mouth.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LesHallesSpawn {
	pub player: Vec3,
	pub npc: Vec3,
	pub look_yaw: f32,
	/// Capsule-center Y on storey 0 and 1. Equal when the stack has one floor.
	pub floor_y: [f32; 2],
}

impl LesHallesSpawn {
	pub fn has_upper(self) -> bool {
		(self.floor_y[1] - self.floor_y[0]).abs() > 1.0
	}
}

impl Default for LesHallesSpawn {
	fn default() -> Self {
		let h = player::capsule_spawn_height();
		Self {
			player: Vec3::new(0.0, h, 0.0),
			npc: Vec3::new(-6.5, h, -8.0),
			look_yaw: -std::f32::consts::FRAC_PI_2,
			floor_y: [h, h],
		}
	}
}

/// Floor count and stair-mouth spawn for one seed in [`ARENA_EXTENT`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArenaPlan {
	pub floors: usize,
	pub spawn: LesHallesSpawn,
	pub failed: bool,
}

/// Inserted when a development (or a pad-only fallback) is mounted.
#[derive(Resource, Clone, Copy, Debug)]
pub struct ArenaMount {
	pub floors: usize,
	pub failed: bool,
}

/// Fit the fixed envelope. The same seed round-trips floor count and spawn XZ.
pub fn spawn_arena(seed: i32) -> ArenaPlan {
	match fit(seed) {
		Ok(dev) => plan_from(&dev),
		Err(_) => ArenaPlan { floors: 0, spawn: LesHallesSpawn::default(), failed: true },
	}
}

/// Pad plus a seed-fit Les Halles stack at the origin. Fit failure leaves the pad.
pub fn mount_arena(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	seed: i32,
) -> ArenaPlan {
	spawn_pad(commands, meshes, materials);
	spawn_development(commands, seed)
}

/// Fit and spawn the stack. The firing range calls this after [`spawn_pad`].
pub fn spawn_development(commands: &mut Commands, seed: i32) -> ArenaPlan {
	let plan = match fit(seed) {
		Ok(dev) => {
			let plan = plan_from(&dev);
			insert_hosts(commands, &dev);
			if plan.floors < 2 {
				bevy::log::warn!(
					"Les Halles stack has {} storey(s); NPC multi-level follow needs at least two",
					plan.floors
				);
			}
			plan
		}
		Err(err) => {
			bevy::log::error!("Les Halles development fit failed: {err}");
			ArenaPlan { floors: 0, spawn: LesHallesSpawn::default(), failed: true }
		}
	};
	commands.insert_resource(plan.spawn);
	commands.insert_resource(ArenaMount { floors: plan.floors, failed: plan.failed });
	plan
}

fn fit(seed: i32) -> Result<MixedUseLesHallesDevelopment, FitError> {
	let bounds = confines_bounds(ARENA_EXTENT);
	let confines = Confines::new(bounds, 0.0, Openings::new());
	let noise = NoiseParams { seed, ..NoiseParams::default() };
	MixedUseLesHallesDevelopment::fit_to_confines(&confines, noise).map(|(dev, _)| dev)
}

fn confines_bounds(extent: Vec3) -> Aabb3d {
	let hx = extent.x.max(1e-4) * 0.5;
	let hz = extent.z.max(1e-4) * 0.5;
	let h = extent.y.max(1e-4);
	Aabb3d::from_min_max(Vec3::new(-hx, 0.0, -hz), Vec3::new(hx, h, hz))
}

fn plan_from(dev: &MixedUseLesHallesDevelopment) -> ArenaPlan {
	let transform = Transform::IDENTITY;
	let n_floors = dev.tower.floors.len();
	let mut walk_y = [0.0f32; 2];
	for (id, floor) in dev.tower.floors.iter().enumerate() {
		let storey = circulation_from_storey(id as u32, floor, transform);
		if id < 2 {
			walk_y[id] = storey.floor_y;
		}
	}
	if n_floors < 2 {
		walk_y[1] = walk_y[0];
	}

	let mut spawn = LesHallesSpawn::default();
	let mut placed_near_stair = false;
	if n_floors >= 2 {
		let mut stair_k = 0usize;
		let last_well_i = n_floors - 2;
		for floor_i in 0..=last_well_i {
			let n_shafts = dev
				.tower
				.floors
				.get(floor_i)
				.map(|floor| floor.floor_plan().shaft_bounds.len())
				.unwrap_or(0);
			for _ in 0..n_shafts {
				let Some(stairwell) = dev.stairwells.get(stair_k) else {
					break;
				};
				let link = circulation_from_stairwell(
					floor_i as u32,
					floor_i as u32 + 1,
					stairwell,
					transform,
				);
				if !placed_near_stair {
					spawn = spawn_from_link(&link, walk_y);
					placed_near_stair = true;
				}
				stair_k += 1;
			}
		}
	}

	if n_floors < 2 || !placed_near_stair {
		if n_floors >= 2 {
			let h = player::capsule_spawn_height();
			let lift = h - walk_y[0];
			spawn.floor_y = [h, walk_y[1] + lift];
		}
	}

	ArenaPlan { floors: n_floors, spawn, failed: false }
}

fn insert_hosts(commands: &mut Commands, dev: &MixedUseLesHallesDevelopment) {
	let transform = Transform::IDENTITY;
	let n_floors = dev.tower.floors.len();
	for (id, floor) in dev.tower.floors.iter().enumerate() {
		let bounds = building_bounds(floor);
		let storey = circulation_from_storey(id as u32, floor, transform);
		for entity in spawn_building_components(commands, floor, transform, bounds) {
			spawn_building_walk_colliders(commands, entity, floor, BUILDING_FRICTION);
			commands.entity(entity).insert((storey.clone(), TrainingArena));
		}
	}

	let mut stair_k = 0usize;
	if n_floors >= 2 {
		let last_well_i = n_floors - 2;
		for floor_i in 0..=last_well_i {
			let n_shafts = dev
				.tower
				.floors
				.get(floor_i)
				.map(|floor| floor.floor_plan().shaft_bounds.len())
				.unwrap_or(0);
			for _ in 0..n_shafts {
				let Some(stairwell) = dev.stairwells.get(stair_k) else {
					break;
				};
				let bounds = building_bounds(stairwell);
				let link = circulation_from_stairwell(
					floor_i as u32,
					floor_i as u32 + 1,
					stairwell,
					transform,
				);
				for entity in spawn_building_components(commands, stairwell, transform, bounds) {
					spawn_building_walk_colliders(commands, entity, stairwell, BUILDING_FRICTION);
					commands.entity(entity).insert((link.clone(), TrainingArena));
				}
				stair_k += 1;
			}
		}
	}

	let bounds = building_bounds(&dev.roof);
	for entity in spawn_building_components(commands, &dev.roof, transform, bounds) {
		spawn_building_walk_colliders(commands, entity, &dev.roof, BUILDING_FRICTION);
		commands.entity(entity).insert(TrainingArena);
	}
}

fn spawn_from_link(link: &CirculationStairwell, walk_y: [f32; 2]) -> LesHallesSpawn {
	let h = player::capsule_spawn_height();
	let mouth = Vec3::new(link.mouth.x, h.max(link.mouth.y + 0.15), link.mouth.z);
	let away = {
		let d = Vec3::new(mouth.x, 0.0, mouth.z);
		if d.length_squared() < 1e-4 {
			Vec3::X
		} else {
			d.normalize()
		}
	};
	let player = mouth - away * 2.4;
	let npc = mouth - away * 4.2 + Vec3::new(-away.z, 0.0, away.x) * 1.6;
	let look_yaw = (-away.z).atan2(-away.x);
	let ground_y = h;
	let lift = ground_y - walk_y[0];
	let upper_y = if (walk_y[1] - walk_y[0]).abs() > 0.5 { walk_y[1] + lift } else { ground_y };
	LesHallesSpawn {
		player: Vec3::new(player.x, ground_y, player.z),
		npc: Vec3::new(npc.x, ground_y, npc.z),
		look_yaw,
		floor_y: [ground_y, upper_y],
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn same_seed_round_trips_floor_count_and_spawn_xz() {
		let a = spawn_arena(PLAYGROUND_SEED);
		let b = spawn_arena(PLAYGROUND_SEED);
		assert!(!a.failed, "playground seed should fit the arena envelope");
		assert!(a.floors >= 2, "envelope is sized for two storeys, got {}", a.floors);
		assert_eq!(a.floors, b.floors);
		assert_eq!(a.spawn.player.x, b.spawn.player.x);
		assert_eq!(a.spawn.player.z, b.spawn.player.z);
	}
}
