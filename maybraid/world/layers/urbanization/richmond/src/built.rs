//! [`Built`]: the hosts fitted to a filled development.

use std::marker::PhantomData;

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{bsn, template_value, Name, Transform, Vec3};
use bevy::scene::Scene;
use building_components::scene_children;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use urbanization_layer_model::UrbanSetting;

use crate::artifact::BuiltDevelopment;
use crate::developments::site::DevelopmentSite;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::host::DevelopmentHosts;
/// Buildings fitted to one filled [`RichmondDevelopment`] over ground `G`.
pub struct Built<G> {
	pub development: BuiltDevelopment,
	/// Where people arrive: over the cell center, at the first pad's height.
	pub setting: UrbanSetting,
	pub setting_at: Vec3,
	_ground: PhantomData<fn() -> G>,
}

impl<G> Built<G> {
	pub fn new(development: BuiltDevelopment, setting: UrbanSetting, setting_at: Vec3) -> Self {
		Self { development, setting, setting_at, _ground: PhantomData }
	}

	/// The buildings fitted to development `id`; `None` for an empty one.
	pub fn fit(id: Id, development: &RichmondDevelopment<G>) -> Option<Self> {
		let built = development.built()?;
		let cell = development.cell();
		let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
		let elevation = development.pads().first().map_or(center.y, |pad| pad.height);
		let arrival_radius =
			((cell.max.x - cell.min.x).min(cell.max.z - cell.min.z) * 0.25).clamp(8.0, 128.0);
		let setting = UrbanSetting { id, arrival_radius };
		Some(Self::new(built, setting, Vec3::new(center.x, elevation, center.z)))
	}
}

impl<G: RichmondGround> hcsg::GenerationScheme for Built<G> {
	lod::hcsg_index_scale!(crate::storage::COLUMN_INDEX_SCALE);

	/// The filled developments originating in `region`.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<DevelopmentSite>(region)
			.into_iter()
			.filter(|OriginalId(id)| RichmondDevelopment::<G>::builds_filled(cx, *id))
			.collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (development, bounds) = cx.get_or_generate_stored::<RichmondDevelopment<G>>(id)?;
		Some((Self::fit(id, &development)?, bounds))
	}
}

/// One level: the [`UrbanSetting`] and a nested host per building, each
/// streaming its own levels.
impl<G: RichmondGround> LodScene for Built<G> {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let setting = self.setting;
		let at = Transform::from_translation(self.setting_at);
		let mut children: Vec<Box<dyn Scene>> = vec![Box::new(bsn! {
			template_value(Name::new("urban-setting"))
			template(move |_ctx| Ok(setting))
			template_value(at)
		})];
		children.extend(
			self.development
				.hosts()
				.iter()
				.map(|host| host.scene(Some(setting.id), lod_ref)),
		);
		scene_children(children)
	}
}

#[cfg(test)]
mod scheme_perf_tests {
	use super::*;
	use durham::Durham;
	use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::{UrbanizationKind, UrbanizationSelection};

	type Ground = OnTerrain<Durham>;

	fn naive_built_ids(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<RichmondDevelopment<Ground>>(region)
			.into_iter()
			.filter(|OriginalId(id)| {
				cx.get_or_generate::<RichmondDevelopment<Ground>>(*id)
					.is_some_and(|development| development.is_filled())
			})
			.collect()
	}

	#[test]
	fn built_discovery_matches_materializing_every_development() -> anyhow::Result<()> {
		let storage = HcsgStorage::default();
		storage.seed(
			crate::DevelopmentConfig {
				sites: crate::DevelopmentSites::Urbanization,
				..crate::DevelopmentConfig::default()
			},
			universal_bounds(),
		);
		storage.seed(
			UrbanizationSelection {
				kind: Some(UrbanizationKind::Frontier),
				..UrbanizationSelection::default()
			},
			universal_bounds(),
		);
		let region = urbanization_cells::UrbanizationExtent::xz_radius_aabb(
			Vec3::ZERO,
			urbanization_cells::DEVELOPMENT_GENERATE_RADIUS_M,
		);
		let mut naive_cx = GenerationContext::new(&storage);
		let mut optimized_cx = GenerationContext::new(&storage);
		let naive = naive_built_ids(&mut naive_cx, region);
		let optimized = optimized_cx.original_ids_for::<Built<Ground>>(region);
		assert_eq!(naive, optimized);
		Ok(())
	}
}
