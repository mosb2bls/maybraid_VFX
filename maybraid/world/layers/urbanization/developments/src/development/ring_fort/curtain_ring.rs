//! Les Halles courtyard ring sampled as a load-bearing curtain wall.

use std::ops::Deref;

use bevy_math::Vec3;
use buildings::{Confines, FillableRegions, FitError, LesHallesFloorPlan, MixedUseLesHallesStorey};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::development::les_halles::{MixedUseLesHallesDevelopment, MixedUseLesHallesHost};

/// Courtyard ring whose gallery is deep enough to carry corner keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct CurtainRing {
	pub halles: MixedUseLesHallesDevelopment,
}

impl Deref for CurtainRing {
	type Target = MixedUseLesHallesDevelopment;

	fn deref(&self) -> &Self::Target {
		&self.halles
	}
}

impl CurtainRing {
	pub fn fit(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (halles, regions) = MixedUseLesHallesDevelopment::fit_curtain(confines, noise)?;
		Ok((Self { halles }, regions))
	}

	pub fn last_plan(&self) -> Option<&LesHallesFloorPlan> {
		self.halles.tower.floors.last().map(MixedUseLesHallesStorey::floor_plan)
	}

	pub fn gallery_width(&self) -> f32 {
		self.last_plan().map(|plan| plan.parameterized.gallery_width).unwrap_or(0.0)
	}

	/// Eave-height center of the gallery-corner square that can carry a keep.
	pub fn keep_anchor(&self, sx: f32, sz: f32) -> Option<Vec3> {
		let plan = self.last_plan()?;
		let gallery = plan.parameterized.gallery_width;
		let hx = (plan.outer.x * 0.5 - gallery * 0.5).max(0.0);
		let hz = (plan.outer.y * 0.5 - gallery * 0.5).max(0.0);
		let eave_y = plan.center_xz.y + plan.storey_height;
		Some(Vec3::new(plan.center_xz.x + sx * hx, eave_y, plan.center_xz.z + sz * hz))
	}

	pub fn hosts_without_roof(&self) -> Vec<MixedUseLesHallesHost> {
		self.halles
			.hosts()
			.into_iter()
			.filter(|host| !matches!(host, MixedUseLesHallesHost::Roof(_)))
			.collect()
	}

	pub fn with_finish(mut self, wall: MaterialRef, roof: MaterialRef) -> Self {
		self.halles = self.halles.with_finish(wall, roof);
		self
	}
}
