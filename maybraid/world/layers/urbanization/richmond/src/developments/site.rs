//! [`DevelopmentSite`]: which development a cell gets, before any ground is read.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Resource;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use procedural_common::SeededHash;
use urbanization_cells::{
	select_cell, select_cell_as, SelectedUrbanization, UrbanDevelopmentKind, UrbanizationExtent,
	UrbanizationSelection,
};
use urbanization_developments::{cell_salt, PadPlan, SiteGround};

use crate::cell::{cell_selected, DevelopmentExtent};
use crate::config::{DevelopmentConfig, DevelopmentSites};
use crate::storage::{column_bounds, overlaps_xz};

/// Fill kind for one development cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentKind {
	Empty,
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
}

impl From<UrbanDevelopmentKind> for DevelopmentKind {
	fn from(kind: UrbanDevelopmentKind) -> Self {
		use UrbanDevelopmentKind as U;
		match kind {
			U::Empty => Self::Empty,
			U::LesHalles => Self::LesHalles,
			U::ShepherdsVillage => Self::ShepherdsVillage,
			U::ShepherdsCommune => Self::ShepherdsCommune,
			U::RingFort => Self::RingFort,
			U::TempleComplex => Self::TempleComplex,
			U::SingleHighrise => Self::SingleHighrise,
			U::SuburbanHomes => Self::SuburbanHomes,
			U::WizardsTower => Self::WizardsTower,
			U::SkybridgeBazaar => Self::SkybridgeBazaar,
			U::OldCityMarket => Self::OldCityMarket,
		}
	}
}

impl DevelopmentKind {
	/// Every authored fill. Occupancy may still pick [`Self::Empty`].
	pub const FILLED: [Self; 10] = [
		Self::LesHalles,
		Self::ShepherdsVillage,
		Self::ShepherdsCommune,
		Self::RingFort,
		Self::TempleComplex,
		Self::SingleHighrise,
		Self::SuburbanHomes,
		Self::WizardsTower,
		Self::SkybridgeBazaar,
		Self::OldCityMarket,
	];

	/// Weighted pick that always returns a building, never [`Self::Empty`].
	pub fn pick_filled(cell: Aabb3d, config: &DevelopmentConfig) -> Self {
		weighted_pick(cell, config).unwrap_or(Self::SingleHighrise)
	}

	/// [`Self::FILLED`] starting at [`Self::pick_filled`], wrapping around.
	pub fn filled_from_pick(cell: Aabb3d, config: &DevelopmentConfig) -> Vec<Self> {
		let preferred = Self::pick_filled(cell, config);
		let start = Self::FILLED.iter().position(|kind| *kind == preferred).unwrap_or(0);
		let count = Self::FILLED.len();
		(0..count).map(|i| Self::FILLED[(start + i) % count]).collect()
	}
}

/// Occupancy, then a weighted kind pick, for one lattice cell.
pub fn select_kind(cell: Aabb3d, config: &DevelopmentConfig) -> DevelopmentKind {
	if !cell_selected(cell, config.occupancy_seed(), config.likelihood, config.spatial_correlation)
	{
		return DevelopmentKind::Empty;
	}
	weighted_pick(cell, config).unwrap_or(DevelopmentKind::Empty)
}

fn weighted_pick(cell: Aabb3d, config: &DevelopmentConfig) -> Option<DevelopmentKind> {
	let weighted = [
		(DevelopmentKind::LesHalles, config.les_halles_weight),
		(DevelopmentKind::ShepherdsVillage, config.shepherds_village_weight),
		(DevelopmentKind::ShepherdsCommune, config.shepherds_commune_weight),
		(DevelopmentKind::RingFort, config.ring_fort_weight),
		(DevelopmentKind::TempleComplex, config.temple_complex_weight),
		(DevelopmentKind::SingleHighrise, config.single_highrise_weight),
		(DevelopmentKind::SuburbanHomes, config.suburban_homes_weight),
		(DevelopmentKind::WizardsTower, config.wizards_tower_weight),
		(DevelopmentKind::SkybridgeBazaar, config.skybridge_bazaar_weight),
		(DevelopmentKind::OldCityMarket, config.old_city_market_weight),
	];
	let total: f32 = weighted.iter().map(|(_, weight)| weight.max(0.0)).sum();
	if total <= f32::EPSILON {
		return None;
	}
	let hash = SeededHash::new(config.seed.wrapping_add(cell_salt(cell)));
	let mut pick = hash.unit(44) * total;
	for (kind, weight) in weighted {
		let weight = weight.max(0.0);
		if weight > 0.0 && pick < weight {
			return Some(kind);
		}
		pick -= weight;
	}
	Some(DevelopmentKind::OldCityMarket)
}

lod::seeded_root!(DevelopmentConfig);

/// Courtyard an authored single-terrace development is re-padded as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AuthoredCourtyard {
	/// Band between the building footprint and the wall.
	pub margin: f32,
	/// Largest courtyard half extent on either axis.
	pub max_half: f32,
	/// Flatten past the wall so its base never meets the ease.
	pub overhang: f32,
	pub ease: f32,
}

/// One development a mode places by hand instead of by occupancy.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoredDevelopment {
	pub cell: Aabb3d,
	/// Kinds tried in order; the first that fits is built.
	pub kinds: Vec<DevelopmentKind>,
	/// Height of the level, dry ground the development is planned on.
	pub height: f32,
	pub config: DevelopmentConfig,
	pub courtyard: Option<AuthoredCourtyard>,
}

impl AuthoredDevelopment {
	pub fn id(&self) -> Id {
		Id::from_cell(self.cell)
	}
}

/// An authored site is planned as if on level ground at its height, away from water.
impl SiteGround for AuthoredDevelopment {
	fn height_at(&mut self, _x: f32, _z: f32) -> Option<f32> {
		Some(self.height)
	}

	fn hydro_overlaps(&mut self, _pad: &PadPlan) -> bool {
		false
	}
}

/// Root input: developments placed by a mode. Procedural sites they overlap stay empty.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct AuthoredDevelopments(pub Vec<AuthoredDevelopment>);

lod::seeded_root!(AuthoredDevelopments);

impl AuthoredDevelopments {
	pub fn get(&self, id: Id) -> Option<&AuthoredDevelopment> {
		self.0.iter().find(|authored| authored.id() == id)
	}

	fn overlapping(&self, region: Aabb3d) -> impl Iterator<Item = &AuthoredDevelopment> + '_ {
		self.0.iter().filter(move |authored| overlaps_xz(region, authored.cell))
	}
}

/// The development planned for one cell: an urbanization leaf, a lattice
/// tile, or an [`AuthoredDevelopment`].
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopmentSite {
	pub cell: Aabb3d,
	pub kind: DevelopmentKind,
	pub authored: Option<AuthoredDevelopment>,
}

/// Urbanization leaf ids for `region` from a session selection, without
/// materializing [`SelectedUrbanization`] in storage.
fn urbanization_leaf_ids(region: Aabb3d, selection: &UrbanizationSelection) -> Vec<OriginalId> {
	UrbanizationExtent::cells_overlapping(region)
		.into_iter()
		.flat_map(|extent| {
			let selected = match selection.kind {
				Some(kind) => select_cell_as(extent, selection.noise, kind),
				None => select_cell(extent, selection.noise),
			};
			selected.leaves.into_iter().filter(|leaf| {
				leaf.kind != UrbanDevelopmentKind::Empty && overlaps_xz(region, leaf.bounds)
			})
		})
		.map(|leaf| OriginalId(leaf.id()))
		.collect()
}

impl DevelopmentSite {
	pub fn is_filled(&self) -> bool {
		self.kind != DevelopmentKind::Empty
	}

	/// Kinds to try, in order.
	pub fn kinds(&self) -> Vec<DevelopmentKind> {
		match &self.authored {
			Some(authored) => authored.kinds.clone(),
			None => vec![self.kind],
		}
	}
}

impl hcsg::GenerationScheme for DevelopmentSite {
	lod::hcsg_index_scale!(crate::storage::COLUMN_INDEX_SCALE);

	/// Authored sites, then the procedural sites of the configured mode.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let Some(config) = cx.get::<DevelopmentConfig>(Id::Universal) else {
			return Vec::new();
		};
		let authored = cx.get::<AuthoredDevelopments>(Id::Universal).unwrap_or_default();
		let mut ids: Vec<OriginalId> =
			authored.overlapping(region).map(|authored| OriginalId(authored.id())).collect();
		match config.sites {
			DevelopmentSites::Authored => {}
			DevelopmentSites::Lattice => {
				ids.extend(DevelopmentExtent::original_ids_overlapping(region));
			}
			DevelopmentSites::Urbanization => {
				if let Some(selection) = cx.get::<UrbanizationSelection>(Id::Universal) {
					ids.extend(urbanization_leaf_ids(region, selection.as_ref()));
				}
			}
		}
		ids.sort_unstable_by_key(|OriginalId(id)| *id);
		ids.dedup();
		ids
	}

	/// An authored site as authored; a procedural one picks its kind, and
	/// stays empty under an authored site.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let config = cx.get::<DevelopmentConfig>(Id::Universal)?;
		let authored = cx.get::<AuthoredDevelopments>(Id::Universal).unwrap_or_default();
		if let Some(entry) = authored.get(id) {
			let kind = entry.kinds.first().copied().unwrap_or(DevelopmentKind::Empty);
			let site = Self { cell: entry.cell, kind, authored: Some(entry.clone()) };
			return Some((site, column_bounds(entry.cell)));
		}
		let cell = id.origin_cell_bounds()?;
		let kind = match config.sites {
			DevelopmentSites::Authored => return None,
			DevelopmentSites::Lattice => {
				DevelopmentExtent::from_id(id)?;
				select_kind(cell, &config)
			}
			DevelopmentSites::Urbanization => {
				let extent = UrbanizationExtent::owning_leaf(id)?;
				let selected = cx.get_or_generate::<SelectedUrbanization>(extent.id())?;
				DevelopmentKind::from(selected.leaf(id)?.kind)
			}
		};
		let kind =
			if authored.overlapping(cell).next().is_some() { DevelopmentKind::Empty } else { kind };
		Some((Self { cell, kind, authored: None }, column_bounds(cell)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::DevelopmentExtent;
	use lod::hcsg::HcsgStorage;

	#[test]
	fn pick_filled_never_returns_empty() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		for seed in 0..48u32 {
			let config = DevelopmentConfig { seed, ..DevelopmentConfig::default() };
			assert_ne!(DevelopmentKind::pick_filled(cell, &config), DevelopmentKind::Empty);
		}
	}

	#[test]
	fn pick_filled_is_seeded_and_not_only_les_halles() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		let config = DevelopmentConfig { seed: 42, ..Default::default() };
		assert_eq!(
			DevelopmentKind::pick_filled(cell, &config),
			DevelopmentKind::pick_filled(cell, &config)
		);
		let kinds: Vec<_> = (0..32u32)
			.map(|seed| {
				DevelopmentKind::pick_filled(
					cell,
					&DevelopmentConfig { seed, ..DevelopmentConfig::default() },
				)
			})
			.collect();
		assert!(
			kinds.iter().any(|kind| *kind != DevelopmentKind::LesHalles),
			"expected a non-Les-Halles pick in 32 seeds, got {kinds:?}"
		);
	}

	fn only(kind: DevelopmentKind) -> DevelopmentConfig {
		let mut config = DevelopmentConfig {
			likelihood: 1.0,
			les_halles_weight: 0.0,
			shepherds_village_weight: 0.0,
			shepherds_commune_weight: 0.0,
			ring_fort_weight: 0.0,
			temple_complex_weight: 0.0,
			single_highrise_weight: 0.0,
			suburban_homes_weight: 0.0,
			wizards_tower_weight: 0.0,
			skybridge_bazaar_weight: 0.0,
			old_city_market_weight: 0.0,
			..DevelopmentConfig::default()
		};
		match kind {
			DevelopmentKind::LesHalles => config.les_halles_weight = 1.0,
			DevelopmentKind::ShepherdsVillage => config.shepherds_village_weight = 1.0,
			DevelopmentKind::ShepherdsCommune => config.shepherds_commune_weight = 1.0,
			DevelopmentKind::RingFort => config.ring_fort_weight = 1.0,
			DevelopmentKind::TempleComplex => config.temple_complex_weight = 1.0,
			DevelopmentKind::SingleHighrise => config.single_highrise_weight = 1.0,
			DevelopmentKind::SuburbanHomes => config.suburban_homes_weight = 1.0,
			DevelopmentKind::WizardsTower => config.wizards_tower_weight = 1.0,
			DevelopmentKind::SkybridgeBazaar => config.skybridge_bazaar_weight = 1.0,
			DevelopmentKind::OldCityMarket => config.old_city_market_weight = 1.0,
			DevelopmentKind::Empty => {}
		}
		config
	}

	#[test]
	fn every_kind_weight_is_independently_selectable() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		for kind in DevelopmentKind::FILLED {
			assert_eq!(select_kind(cell, &only(kind)), kind);
		}
	}

	fn seeded(config: DevelopmentConfig, authored: AuthoredDevelopments) -> HcsgStorage {
		let storage = HcsgStorage::default();
		storage.seed(config, lod::hcsg::universal_bounds());
		storage.seed(authored, lod::hcsg::universal_bounds());
		storage
	}

	#[test]
	fn lattice_sites_follow_occupancy() -> anyhow::Result<()> {
		let config = DevelopmentConfig {
			sites: DevelopmentSites::Lattice,
			..only(DevelopmentKind::RingFort)
		};
		let storage = seeded(config, AuthoredDevelopments::default());
		let cell = DevelopmentExtent::from_cell_index(2, -1);
		let mut cx = lod::hcsg::GenerationContext::new(&storage);
		let site = cx
			.get_or_generate::<DevelopmentSite>(cell.id())
			.ok_or_else(|| anyhow::anyhow!("site"))?;
		anyhow::ensure!(site.kind == DevelopmentKind::RingFort);
		anyhow::ensure!(site.cell == cell.aabb());
		Ok(())
	}

	#[test]
	fn authored_sites_replace_the_procedural_ones_they_overlap() -> anyhow::Result<()> {
		let config = DevelopmentConfig {
			sites: DevelopmentSites::Lattice,
			..only(DevelopmentKind::RingFort)
		};
		let lattice = DevelopmentExtent::from_cell_index(0, 0);
		let cell = Aabb3d::from_min_max(
			bevy::math::Vec3::new(100.0, 0.0, 100.0),
			bevy::math::Vec3::new(200.0, 1.0, 200.0),
		);
		let authored = AuthoredDevelopment {
			cell,
			kinds: vec![DevelopmentKind::LesHalles],
			height: 4.0,
			config: config.clone(),
			courtyard: None,
		};
		let storage = seeded(config, AuthoredDevelopments(vec![authored.clone()]));
		let mut cx = lod::hcsg::GenerationContext::new(&storage);
		let ids = cx.original_ids_for::<DevelopmentSite>(lattice.aabb());
		anyhow::ensure!(ids.contains(&OriginalId(authored.id())), "authored site is an origin");
		let site = cx
			.get_or_generate::<DevelopmentSite>(authored.id())
			.ok_or_else(|| anyhow::anyhow!("authored site"))?;
		anyhow::ensure!(site.kind == DevelopmentKind::LesHalles && site.authored.is_some());
		let procedural = cx
			.get_or_generate::<DevelopmentSite>(lattice.id())
			.ok_or_else(|| anyhow::anyhow!("lattice site"))?;
		anyhow::ensure!(procedural.kind == DevelopmentKind::Empty, "overlapped site stays empty");
		Ok(())
	}

	mod scheme_perf_tests {
		use super::*;
		use bevy::math::Vec3;
		use lod::hcsg::{universal_bounds, GenerationContext};
		use procedural_common::NoiseParams;
		use std::time::Instant;
		use urbanization_cells::{
			UrbanizationKind, UrbanizationSelection, DEVELOPMENT_GENERATE_RADIUS_M,
		};

		fn urbanization_config() -> DevelopmentConfig {
			DevelopmentConfig {
				sites: DevelopmentSites::Urbanization,
				..DevelopmentConfig::default()
			}
		}

		fn seed_urbanization(storage: &HcsgStorage) {
			storage.seed(urbanization_config(), universal_bounds());
			storage.seed(
				UrbanizationSelection {
					noise: NoiseParams::from_scalar(1337.0, 0.0005, 1.0, 1),
					kind: Some(UrbanizationKind::Frontier),
				},
				universal_bounds(),
			);
		}

		fn discovery_region() -> Aabb3d {
			UrbanizationExtent::xz_radius_aabb(Vec3::ZERO, DEVELOPMENT_GENERATE_RADIUS_M)
		}

		/// Discovery path that materialized every overlapping selection in storage.
		fn naive_urbanization_site_ids(
			cx: &mut GenerationContext,
			region: Aabb3d,
		) -> Vec<OriginalId> {
			let authored = cx.get::<AuthoredDevelopments>(Id::Universal).unwrap_or_default();
			let mut ids: Vec<OriginalId> =
				authored.overlapping(region).map(|authored| OriginalId(authored.id())).collect();
			for extent in UrbanizationExtent::cells_overlapping(region) {
				let Some(selected) = cx.get_or_generate::<SelectedUrbanization>(extent.id()) else {
					continue;
				};
				ids.extend(
					selected
						.leaves
						.iter()
						.filter(|leaf| {
							leaf.kind != UrbanDevelopmentKind::Empty
								&& overlaps_xz(region, leaf.bounds)
						})
						.map(|leaf| OriginalId(leaf.id())),
				);
			}
			ids.sort_unstable_by_key(|OriginalId(id)| *id);
			ids.dedup();
			ids
		}

		#[test]
		fn urbanization_site_discovery_matches_materialized_selections() -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			seed_urbanization(&storage);
			let region = discovery_region();
			let mut naive_cx = GenerationContext::new(&storage);
			let mut grouped_cx = GenerationContext::new(&storage);
			let naive = naive_urbanization_site_ids(&mut naive_cx, region);
			let grouped = grouped_cx.original_ids_for::<DevelopmentSite>(region);
			assert_eq!(naive, grouped);
			assert!(!grouped.is_empty());
			Ok(())
		}

		/// `cargo test -p richmond urbanization_site_discovery_microbench --release -- --ignored --nocapture`
		#[test]
		#[ignore = "microbench"]
		fn urbanization_site_discovery_microbench() {
			let region = discovery_region();
			let runs = 40;

			let bench = |materialize: bool| -> std::time::Duration {
				let start = Instant::now();
				for _ in 0..runs {
					let storage = HcsgStorage::default();
					seed_urbanization(&storage);
					let mut cx = GenerationContext::new(&storage);
					if materialize {
						naive_urbanization_site_ids(&mut cx, region);
					} else {
						cx.original_ids_for::<DevelopmentSite>(region);
					}
				}
				start.elapsed()
			};

			let materialize = bench(true);
			let local = bench(false);
			let materialize_ms = materialize.as_secs_f64() * 1000.0 / runs as f64;
			let local_ms = local.as_secs_f64() * 1000.0 / runs as f64;
			eprintln!(
				"urbanization_site_discovery_microbench ({} m generate ring, Frontier): \
				 materialize={materialize_ms:.2} ms/discover local={local_ms:.2} ms/discover ({:.2}x)",
				DEVELOPMENT_GENERATE_RADIUS_M,
				materialize_ms / local_ms.max(1e-9)
			);
			assert!(local < materialize, "local selection should beat storage materialization");
		}
	}
}
