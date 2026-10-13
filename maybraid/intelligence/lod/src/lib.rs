//! Viewer-axis work LOD among plants that already exist.
//!
//! World bakes [`IntelligenceLod`] and [`IntelligencePriority`]. Personality
//! crates only read. Missing lod is [`IntelligenceBand::Near`]. Look / aim
//! promotion uses FOV magnification inside [`IntelligenceLook`] plus the
//! [`IntelligenceFocus`] mailbox. Mailbox apply reads [`IntelligenceLookFrame`]
//! every Update (wider inset, no `near_m` cap).

mod focus;

use std::collections::HashMap;

use bevy::prelude::*;

pub use focus::{
	fov_magnification, look_applies, look_near_m, look_promotes, IntelligenceFocus,
	IntelligenceFocusSample, IntelligenceLook, IntelligenceLookFrame, LOOK_APPLY_FOV_INSET,
	LOOK_FOV_INSET,
};

/// Viewer-axis work band. Near work is spent first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntelligenceBand {
	#[default]
	Near = 0,
	Mid = 1,
	Far = 2,
}

impl IntelligenceBand {
	/// Combat / evade, or closer than this, is Near.
	pub const NEAR_M: f32 = 80.0;
	/// Ignore plants between [`Self::NEAR_M`] and this are Mid.
	pub const MID_M: f32 = 200.0;

	/// Distance + tactic + look cut used by the world bake pulse.
	///
	/// `look` is inset-FOV / focus-mailbox inside `NEAR_M * fov_magnification`.
	pub fn from_viewer(dist: f32, combat_or_evade: bool, look: bool) -> Self {
		if combat_or_evade || dist < Self::NEAR_M || look {
			Self::Near
		} else if dist < Self::MID_M {
			Self::Mid
		} else {
			Self::Far
		}
	}

	/// Stretch discovery / scan intervals for Mid and Far.
	pub fn interval_scale(self) -> f32 {
		match self {
			Self::Near => 1.0,
			Self::Mid => 2.0,
			Self::Far => 4.0,
		}
	}

	/// Shrink per-tick candidate / sample counts. Far keeps one.
	pub fn scale_count(self, n: usize) -> usize {
		match self {
			Self::Near => n,
			Self::Mid => n.max(1) / 2,
			Self::Far => 1,
		}
	}
}

/// Sort key for the world bake pulse (`band`, fairness inversion, entity bits).
pub type IntelligenceBakeKey = (IntelligenceBand, u8, u64);

/// Per-plant work LOD. Insert once; the world pulse writes `band` / `skips`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntelligenceLod {
	pub band: IntelligenceBand,
	pub skips: u8,
}

impl IntelligenceLod {
	pub const FAIRNESS_CAP: u8 = 8;

	pub fn missing() -> Self {
		Self { band: IntelligenceBand::Near, skips: 0 }
	}

	pub fn band_or_near(lod: Option<&Self>) -> IntelligenceBand {
		lod.map(|lod| lod.band).unwrap_or(IntelligenceBand::Near)
	}

	/// Bake sort key: Near, then high-`skips` Mid/Far, then entity bits.
	pub fn bake_key(self, entity: Entity) -> IntelligenceBakeKey {
		(self.band, Self::FAIRNESS_CAP.saturating_sub(self.skips), entity.to_bits())
	}
}

impl Default for IntelligenceLod {
	fn default() -> Self {
		Self::missing()
	}
}

/// Shared drain order. Rebuilt on the world bake pulse only.
#[derive(Resource, Clone, Debug, Default)]
pub struct IntelligencePriority {
	/// Near / high-`skips` first.
	pub rank: HashMap<Entity, u32>,
}

impl IntelligencePriority {
	pub fn rank_of(&self, entity: Entity) -> u32 {
		self.rank.get(&entity).copied().unwrap_or(u32::MAX)
	}
}

/// Sort a due set by shared rank. Do not walk `priority.rank` with `query.get`.
pub fn due_by_rank(due: &mut [Entity], priority: &IntelligencePriority) {
	due.sort_by_key(|entity| (priority.rank_of(*entity), entity.to_bits()));
}

/// Per-entity jitter in `[0.8, 1.2]×` for scan / forget clocks so plants do not align.
pub fn staggered_interval(interval: f32, entity: Entity, salt: u64) -> f32 {
	let bits = entity.to_bits().wrapping_add(salt.wrapping_mul(0x9e37_79b9));
	let jitter = (bits % 1_001) as f32 / 1_000.0;
	interval.max(0.05) * (0.8 + jitter * 0.4)
}

/// First Mid/Far due actor that has waited long enough.
pub fn reserve_fairness(due: &[Entity], lods: &Query<&IntelligenceLod>) -> Option<Entity> {
	due.iter().copied().find(|entity| {
		lods.get(*entity).is_ok_and(|lod| {
			lod.band != IntelligenceBand::Near && lod.skips >= IntelligenceLod::FAIRNESS_CAP
		})
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn missing_lod_is_near() {
		assert_eq!(IntelligenceLod::band_or_near(None), IntelligenceBand::Near);
		assert_eq!(IntelligenceLod::missing().band, IntelligenceBand::Near);
		assert_eq!(IntelligenceLod::missing().skips, 0);
	}

	#[test]
	fn viewer_cuts_and_combat_are_near() {
		assert_eq!(IntelligenceBand::from_viewer(40.0, false, false), IntelligenceBand::Near);
		assert_eq!(IntelligenceBand::from_viewer(120.0, false, false), IntelligenceBand::Mid);
		assert_eq!(IntelligenceBand::from_viewer(300.0, false, false), IntelligenceBand::Far);
		assert_eq!(IntelligenceBand::from_viewer(300.0, true, false), IntelligenceBand::Near);
		assert_eq!(IntelligenceBand::from_viewer(120.0, false, true), IntelligenceBand::Near);
		assert_eq!(IntelligenceBand::from_viewer(300.0, false, true), IntelligenceBand::Near);
	}

	#[test]
	fn interval_and_count_scale_with_band() {
		assert_eq!(IntelligenceBand::Near.interval_scale(), 1.0);
		assert_eq!(IntelligenceBand::Mid.interval_scale(), 2.0);
		assert_eq!(IntelligenceBand::Far.interval_scale(), 4.0);
		assert_eq!(IntelligenceBand::Near.scale_count(8), 8);
		assert_eq!(IntelligenceBand::Mid.scale_count(8), 4);
		assert_eq!(IntelligenceBand::Far.scale_count(8), 1);
		assert_eq!(IntelligenceBand::Mid.scale_count(1), 0);
	}

	#[test]
	fn due_by_rank_orders_near_before_far() -> anyhow::Result<()> {
		let near = Entity::from_bits(2);
		let mid = Entity::from_bits(3);
		let far = Entity::from_bits(1);
		let mut priority = IntelligencePriority::default();
		priority.rank.insert(near, 0);
		priority.rank.insert(mid, 1);
		priority.rank.insert(far, 2);
		let mut due = vec![far, mid, near];
		due_by_rank(&mut due, &priority);
		anyhow::ensure!(due == [near, mid, far]);
		Ok(())
	}

	#[test]
	fn bake_key_puts_high_skips_ahead_of_fresh_far() {
		let fresh = Entity::from_bits(1);
		let waiting = Entity::from_bits(2);
		let fresh_lod = IntelligenceLod { band: IntelligenceBand::Far, skips: 0 };
		let waiting_lod =
			IntelligenceLod { band: IntelligenceBand::Far, skips: IntelligenceLod::FAIRNESS_CAP };
		assert!(waiting_lod.bake_key(waiting) < fresh_lod.bake_key(fresh));
		let near = IntelligenceLod::missing();
		assert!(near.bake_key(Entity::from_bits(9)) < waiting_lod.bake_key(waiting));
	}

	#[test]
	fn staggered_interval_is_deterministic_and_bounded() {
		let entity = Entity::from_bits(42);
		let a = staggered_interval(4.0, entity, 0);
		let b = staggered_interval(4.0, entity, 0);
		assert_eq!(a, b);
		assert!(a >= 3.2 && a <= 4.8);
		assert!(staggered_interval(0.0, entity, 1) >= 0.04);
	}

	#[test]
	fn missing_rank_sorts_last() {
		let known = Entity::from_bits(1);
		let unknown = Entity::from_bits(2);
		let mut priority = IntelligencePriority::default();
		priority.rank.insert(known, 0);
		let mut due = vec![unknown, known];
		due_by_rank(&mut due, &priority);
		assert_eq!(due, [known, unknown]);
		assert_eq!(priority.rank_of(unknown), u32::MAX);
	}

	#[test]
	fn reserve_fairness_picks_the_oldest_non_near() -> Result<(), bevy::ecs::system::RunSystemError>
	{
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		let near = world
			.spawn(IntelligenceLod {
				band: IntelligenceBand::Near,
				skips: IntelligenceLod::FAIRNESS_CAP,
			})
			.id();
		let far = world
			.spawn(IntelligenceLod {
				band: IntelligenceBand::Far,
				skips: IntelligenceLod::FAIRNESS_CAP,
			})
			.id();
		world.run_system_once(move |lods: Query<&IntelligenceLod>| {
			assert_eq!(reserve_fairness(&[near, far], &lods), Some(far));
		})?;
		Ok(())
	}
}
