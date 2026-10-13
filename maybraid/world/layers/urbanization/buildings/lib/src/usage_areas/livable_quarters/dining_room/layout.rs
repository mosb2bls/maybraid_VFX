//! Dining room plan packer: host-scaled table + optional filler.

use bevy_math::bounding::Aabb3d;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FitError};
use crate::placer::{
	init_host_with, pack_kinds, CommitEffect, InitHostOpts, KindSpec, PackKnobs, Predicate,
	ProgramTier, ProposeKnobs, SoftGoalRole,
};

pub const MIN_AREA: f32 = 2.2 * 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiningKind {
	Table,
	Filler,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DiningRoomPacked {
	pub tables: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiningRoomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl DiningRoomRegions {
	fn catalog() -> &'static [KindSpec<DiningKind>] {
		static CATALOG: [KindSpec<DiningKind>; 2] = [
			KindSpec {
				id: DiningKind::Table,
				tier: ProgramTier::Appointed,
				weight: 1.0,
				max_count: Some(1),
				soft_goal: SoftGoalRole::Appointed,
				propose: ProposeKnobs::FreeExtentFrac {
					long_frac_min: 0.42,
					long_frac_max: 0.72,
					short_frac_min: 0.28,
					short_frac_max: 0.48,
					height: 0.75,
					prefer_wall: false,
					align_long_to_host: true,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: DiningKind::Filler,
				tier: ProgramTier::Filler,
				weight: 0.4,
				max_count: Some(2),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 0.35,
					max_x: 0.55,
					min_z: 0.35,
					max_z: 0.55,
					height: 0.5,
					prefer_wall: true,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
		];
		&CATALOG
	}

	pub fn pack(
		&self,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<DiningRoomPacked, FitError> {
		let mut host = init_host_with(confines, InitHostOpts { passage_wall_lip: true })?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall { reason: "dining room" });
		}
		let mut packed = DiningRoomPacked::default();
		let placed = pack_kinds(
			Self::catalog(),
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: self.occupancy,
				structure_occupancy: self.occupancy.max(0.7),
				max_steps: 10,
			},
			&mut host,
			confines,
			noise,
		)?;
		for (kind, aabb) in placed {
			match kind {
				DiningKind::Table => packed.tables.push(aabb),
				DiningKind::Filler => packed.fillers.push(aabb),
			}
		}
		if packed.tables.is_empty() {
			return Err(FitError::TooSmall { reason: "dining table" });
		}
		Ok(packed)
	}
}
