//! Sitting room plan packer: compact living layout.

use bevy_math::bounding::Aabb3d;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FitError};
use crate::placer::{
	init_host_with, pack_kinds, CommitEffect, InitHostOpts, KindSpec, PackKnobs, Predicate,
	ProgramTier, ProposeKnobs, SoftGoalRole,
};

pub const MIN_AREA: f32 = 2.2 * 2.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SittingKind {
	PrimarySeating,
	SecondarySeating,
	Filler,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SittingRoomPacked {
	pub primary_seating: Vec<Aabb3d>,
	pub secondary_seating: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SittingRoomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl SittingRoomRegions {
	fn catalog() -> &'static [KindSpec<SittingKind>] {
		static CATALOG: [KindSpec<SittingKind>; 3] = [
			KindSpec {
				id: SittingKind::PrimarySeating,
				tier: ProgramTier::Appointed,
				weight: 1.0,
				max_count: Some(1),
				soft_goal: SoftGoalRole::Appointed,
				propose: ProposeKnobs::WallLongFrac {
					along_frac_min: 0.22,
					along_frac_max: 0.35,
					depth_min: 0.6,
					depth_max: 0.85,
					height: 0.8,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: SittingKind::SecondarySeating,
				tier: ProgramTier::Appointed,
				weight: 0.5,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLongFrac {
					along_frac_min: 0.14,
					along_frac_max: 0.22,
					depth_min: 0.55,
					depth_max: 0.8,
					height: 0.8,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: SittingKind::Filler,
				tier: ProgramTier::Filler,
				weight: 0.35,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 0.35,
					max_x: 0.55,
					min_z: 0.35,
					max_z: 0.55,
					height: 0.5,
					prefer_wall: false,
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
	) -> Result<SittingRoomPacked, FitError> {
		let mut host = init_host_with(confines, InitHostOpts { passage_wall_lip: true })?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall { reason: "sitting room" });
		}
		let mut packed = SittingRoomPacked::default();
		let placed = pack_kinds(
			Self::catalog(),
			PackKnobs {
				spaciousness: self.spaciousness,
				furniture_occupancy: self.occupancy,
				structure_occupancy: self.occupancy.max(0.65),
				max_steps: 14,
			},
			&mut host,
			confines,
			noise,
		)?;
		for (kind, aabb) in placed {
			match kind {
				SittingKind::PrimarySeating => packed.primary_seating.push(aabb),
				SittingKind::SecondarySeating => packed.secondary_seating.push(aabb),
				SittingKind::Filler => packed.fillers.push(aabb),
			}
		}
		if packed.primary_seating.is_empty() {
			return Err(FitError::TooSmall { reason: "sitting room seating" });
		}
		Ok(packed)
	}
}
