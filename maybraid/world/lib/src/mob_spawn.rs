//! `mobs spawn` debug command: place a deterministic grid of mob hosts.
//!
//! Hosts go through [`PendingMobGroups`] as a [`GroupKind::Placed`] group, so
//! they use the same [`MobScene`] install, High fulfill, and intelligence as
//! streamed cells. Rosters come from `seed`, not from the world sample.

use barking::{GroupKind, MobEnvironmentSample, MobGroup, PendingMobGroups, PlacedMob};
use bevy::prelude::*;
use durham::{DurhamSurface, WorldBaseTerrain};
use game_commands::command::CommandConsoleOutput;
use mob_scenes::{MobKind, MobScene};
use world_player::player::holding_elevation;
use world_player::Player;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct RequestSpawnMobs {
	pub kind: MobKind,
	pub count: u32,
	/// Grid center; the player's XZ when `None`.
	pub at: Option<Vec2>,
	pub spacing: f32,
	pub seed: u32,
}

impl RequestSpawnMobs {
	/// Row-major square grid centered on `center`, `spacing` apart.
	pub fn grid(&self, center: Vec2) -> Vec<Vec2> {
		let count = self.count as usize;
		let columns = (count as f32).sqrt().ceil().max(1.0) as usize;
		let rows = count.div_ceil(columns);
		let offset = Vec2::new(columns as f32 - 1.0, rows as f32 - 1.0) * 0.5 * self.spacing;
		(0..count)
			.map(|i| {
				let cell = Vec2::new((i % columns) as f32, (i / columns) as f32);
				center + cell * self.spacing - offset
			})
			.collect()
	}

	/// Roster number of host `index`. Fixed per `(seed, index)` so a density
	/// sweep keeps the same first hosts.
	pub fn roster_num(&self, index: usize) -> f32 {
		self.seed as f32 * 1_000.0 + index as f32 * 7.0 + 1.0
	}
}

pub(crate) fn apply_spawn_mob_requests(
	mut commands: Commands,
	requests: Query<(Entity, &RequestSpawnMobs)>,
	players: Query<&Transform, With<Player>>,
	surface: DurhamSurface,
	base: Res<WorldBaseTerrain>,
	pending: Option<ResMut<PendingMobGroups>>,
	mut console: Option<ResMut<CommandConsoleOutput>>,
) {
	if requests.is_empty() {
		return;
	}
	let Some(mut pending) = pending else {
		for (entity, _) in &requests {
			commands.entity(entity).despawn();
		}
		warn!(target: "maybraid.mobs", "mobs spawn: mob groups are not installed");
		return;
	};
	for (entity, request) in &requests {
		commands.entity(entity).despawn();
		let Some(center) = request
			.at
			.or_else(|| players.single().ok().map(|transform| transform.translation.xz()))
		else {
			warn!(target: "maybraid.mobs", "mobs spawn: no --at and no player");
			continue;
		};
		let mut members = 0;
		let mobs: Vec<PlacedMob> = request
			.grid(center)
			.into_iter()
			.enumerate()
			.map(|(index, xz)| {
				let elevation = match surface.height_at(xz) {
					Ok(Some(elevation)) => elevation,
					Ok(None) | Err(_) => holding_elevation(&base.0, xz.x, xz.y),
				};
				let scene = MobScene::of_kind(request.kind, request.roster_num(index));
				members += scene.mob.roster.members.len();
				PlacedMob {
					scene,
					transform: Transform::from_xyz(xz.x, elevation, xz.y),
					environment: MobEnvironmentSample { elevation: Some(elevation), ..default() },
				}
			})
			.collect();
		let line = format!(
			"mobs spawn {:?}: {} host(s), {members} member(s) around {:.0},{:.0}",
			request.kind,
			mobs.len(),
			center.x,
			center.y
		);
		info!(target: "maybraid.mobs", "{line}");
		if let Some(console) = console.as_mut() {
			console.0 = line;
		}
		pending.push(MobGroup {
			kind: GroupKind::Placed,
			seed: u64::from(request.seed),
			origin: center,
			extent: request.spacing * (request.count as f32).sqrt().ceil(),
			mobs,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn request(count: u32) -> RequestSpawnMobs {
		RequestSpawnMobs { kind: MobKind::Pleb, count, at: None, spacing: 10.0, seed: 1 }
	}

	#[test]
	fn grid_is_centered_and_spaced() {
		let points = request(4).grid(Vec2::new(100.0, -50.0));
		assert_eq!(
			points,
			vec![
				Vec2::new(95.0, -55.0),
				Vec2::new(105.0, -55.0),
				Vec2::new(95.0, -45.0),
				Vec2::new(105.0, -45.0),
			]
		);
	}

	#[test]
	fn grid_keeps_partial_last_row() {
		let points = request(5).grid(Vec2::ZERO);
		assert_eq!(points.len(), 5);
		assert_eq!(points[0], Vec2::new(-10.0, -5.0));
		assert_eq!(points[4], Vec2::new(0.0, 5.0));
	}

	#[test]
	fn roster_numbers_are_stable_across_counts() {
		let small = request(2);
		let large = request(8);
		assert_eq!(small.roster_num(1), large.roster_num(1));
		let a = MobScene::of_kind(MobKind::Pleb, small.roster_num(1));
		let b = MobScene::of_kind(MobKind::Pleb, large.roster_num(1));
		assert_eq!(a.mob.roster, b.mob.roster);
	}
}
