//! Fitting one shepherds' house or hut at a site, with its finish.
//!
//! The village scatters them; other developments place the same buildings as
//! their halls, stalls, homes, and market.

use std::sync::Arc;

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelStyle;
use buildings::{Confines, Fit, Openings};
use procedural_common::{NoiseParams, SeededHash};

use super::building::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};
use crate::finish::{DevelopmentFinish, DevelopmentFinishRole, SuburbanPaletteBias};
use crate::math::lerp;
use crate::scatter::{ScatterChoice, ScatterRecipe};

#[derive(Debug, Clone, Copy)]
pub(crate) enum ShepherdsBuildingKind {
	House,
	Hut,
}

pub(crate) fn shepherds_recipe() -> ScatterRecipe<ShepherdsBuildingKind> {
	ScatterRecipe {
		grid_side: 4,
		min_count: 6,
		max_count: 10,
		cell_inset: 32.0,
		jitter: 6.0,
		clearance: 3.0,
		choices: vec![
			ScatterChoice {
				kind: ShepherdsBuildingKind::House,
				weight: 1.0,
				min_footprint: HOUSE_MIN_FOOTPRINT,
				max_footprint: HOUSE_MAX_FOOTPRINT,
			},
			ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: HUT_MIN_FOOTPRINT,
				max_footprint: HUT_MAX_FOOTPRINT,
			},
		],
	}
}

pub(crate) fn shepherds_authored_height(kind: ShepherdsBuildingKind) -> f32 {
	match kind {
		ShepherdsBuildingKind::House => 2.0 * HOUSE_STOREY_HEIGHT,
		ShepherdsBuildingKind::Hut => HUT_HEIGHT,
	}
}

pub(crate) fn sample_shepherds_kind(hash: SeededHash) -> ShepherdsBuildingKind {
	if hash.unit(1) < 0.5 {
		ShepherdsBuildingKind::House
	} else {
		ShepherdsBuildingKind::Hut
	}
}

pub(crate) fn sample_shepherds_footprint(hash: SeededHash, kind: ShepherdsBuildingKind) -> Vec2 {
	let (lo, hi) = match kind {
		ShepherdsBuildingKind::House => (HOUSE_MIN_FOOTPRINT, HOUSE_MAX_FOOTPRINT),
		ShepherdsBuildingKind::Hut => (HUT_MIN_FOOTPRINT, HUT_MAX_FOOTPRINT),
	};
	Vec2::new(lerp(lo, hi, hash.unit(2)), lerp(lo, hi, hash.unit(3)))
}

pub(crate) fn fit_shepherds_building(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
) -> Option<ShepherdsVillageBuilding> {
	fit_shepherds_building_with_role(kind, center, yaw, footprint, height, hash, noise, None)
}

pub(crate) fn fit_shepherds_building_for_role(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
	role: DevelopmentFinishRole,
) -> Option<ShepherdsVillageBuilding> {
	fit_shepherds_building_with_role(kind, center, yaw, footprint, height, hash, noise, Some(role))
}

pub(crate) fn fit_suburban_building(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
	bias: SuburbanPaletteBias,
) -> Option<ShepherdsVillageBuilding> {
	fit_shepherds_building_with_finish(
		kind,
		center,
		yaw,
		footprint,
		height,
		hash,
		noise,
		Some((DevelopmentFinishRole::SuburbanHome, Some(bias))),
	)
}

fn fit_shepherds_building_with_role(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
	role: Option<DevelopmentFinishRole>,
) -> Option<ShepherdsVillageBuilding> {
	fit_shepherds_building_with_finish(
		kind,
		center,
		yaw,
		footprint,
		height,
		hash,
		noise,
		role.map(|role| (role, None)),
	)
}

fn fit_shepherds_building_with_finish(
	kind: ShepherdsBuildingKind,
	center: Vec2,
	yaw: f32,
	footprint: Vec2,
	height: f32,
	hash: SeededHash,
	noise: NoiseParams,
	finish_role: Option<(DevelopmentFinishRole, Option<SuburbanPaletteBias>)>,
) -> Option<ShepherdsVillageBuilding> {
	let authored_height = shepherds_authored_height(kind);
	let confines = Confines::new(
		Aabb3d::from_min_max(
			Vec3::new(center.x - footprint.x * 0.5, height, center.y - footprint.y * 0.5),
			Vec3::new(
				center.x + footprint.x * 0.5,
				height + authored_height,
				center.y + footprint.y * 0.5,
			),
		),
		yaw,
		Openings::new(),
	);
	let building = match kind {
		ShepherdsBuildingKind::House => {
			let (house, _) = ShepherdsHouse::fit_to_confines(&confines, noise).ok()?;
			let wooden = house.wall_style == PanelStyle::RibAndPlank;
			let finish = finish_role.map_or_else(
				|| DevelopmentFinish::pick_shepherds(hash, wooden),
				|(role, bias)| {
					bias.map_or_else(
						|| DevelopmentFinish::pick_for_role(hash, role, wooden),
						|bias| DevelopmentFinish::pick_suburban_home(hash, bias, wooden),
					)
				},
			);
			ShepherdsBuilding::House(Arc::new(
				house.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		}
		ShepherdsBuildingKind::Hut => {
			let (hut, _) = ShepherdsHut::fit_to_confines(&confines, noise).ok()?;
			let wooden = hut.wall_style == PanelStyle::RibAndPlank;
			let finish = finish_role.map_or_else(
				|| DevelopmentFinish::pick_shepherds(hash, wooden),
				|(role, bias)| {
					bias.map_or_else(
						|| DevelopmentFinish::pick_for_role(hash, role, wooden),
						|bias| DevelopmentFinish::pick_suburban_home(hash, bias, wooden),
					)
				},
			);
			ShepherdsBuilding::Hut(Arc::new(
				hut.with_finish(ShepherdsFinish { wall: finish.wall, roof: finish.roof }),
			))
		}
	};
	Some(ShepherdsVillageBuilding {
		center_xz: center,
		yaw,
		footprint,
		ground_height: height,
		building,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fitted_finish(
		building: &ShepherdsVillageBuilding,
	) -> anyhow::Result<(&ShepherdsFinish, bool)> {
		let (finish, wooden) = match &building.building {
			ShepherdsBuilding::House(house) => {
				(house.finish.as_ref(), house.wall_style == PanelStyle::RibAndPlank)
			}
			ShepherdsBuilding::Hut(hut) => {
				(hut.finish.as_ref(), hut.wall_style == PanelStyle::RibAndPlank)
			}
		};
		finish
			.map(|finish| (finish, wooden))
			.ok_or_else(|| anyhow::anyhow!("fitted shepherds building should have a finish"))
	}

	#[test]
	fn role_aware_fit_selects_role_finish_without_changing_legacy_fit() -> anyhow::Result<()> {
		let hash = SeededHash::new(37);
		let args = (
			ShepherdsBuildingKind::House,
			Vec2::new(20.0, 30.0),
			0.0,
			Vec2::new(18.0, 16.0),
			4.0,
			hash,
			NoiseParams::default(),
		);
		let legacy = fit_shepherds_building(args.0, args.1, args.2, args.3, args.4, args.5, args.6)
			.ok_or_else(|| anyhow::anyhow!("legacy house should fit"))?;
		let role_aware = fit_shepherds_building_for_role(
			args.0,
			args.1,
			args.2,
			args.3,
			args.4,
			args.5,
			args.6,
			DevelopmentFinishRole::SuburbanHome,
		)
		.ok_or_else(|| anyhow::anyhow!("role-aware house should fit"))?;

		let (legacy_finish, legacy_wooden) = fitted_finish(&legacy)?;
		let (role_finish, role_wooden) = fitted_finish(&role_aware)?;
		let expected_legacy = DevelopmentFinish::pick_shepherds(hash, legacy_wooden);
		let expected_role = DevelopmentFinish::pick_for_role(
			hash,
			DevelopmentFinishRole::SuburbanHome,
			role_wooden,
		);
		assert_eq!(legacy_finish.wall, expected_legacy.wall);
		assert_eq!(legacy_finish.roof, expected_legacy.roof);
		assert_eq!(role_finish.wall, expected_role.wall);
		assert_eq!(role_finish.roof, expected_role.roof);
		assert_ne!(legacy_finish, role_finish);
		Ok(())
	}
}
