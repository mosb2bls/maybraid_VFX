//! Read-only geographic facts from stored stamp and watershed records.
//!
//! This query never admits terrain, constructs geometry, or mutates storage.
//! It names each authored source once. Derived [`HydroComplexCell`] bags are
//! not sources.
//!
//! Nothing outside tests reads it since Geneva's naming moved to the shared
//! runtime; it stays compiled so a reader can return to it.
#![cfg_attr(not(test), allow(dead_code))]

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use lod::gen::{Id, Version};
use lod::hcsg::HcsgValue;
use lod::hcsg::{Busy, HcsgStorage};
use procedural_common::Bounds2;

use crate::terrain::stamps::{
	CanyonHighPassStampCell, CanyonLowPassStampCell, MassifHighPassStampCell,
	MassifLowPassStampCell, PlateauHighPassStampCell, PlateauLowPassStampCell,
	PocketWaterHighPassStampCell, PocketWaterLowPassStampCell, RollingHighPassStampCell,
	RollingLowPassStampCell, ValleyHighPassStampCell, ValleyLowPassStampCell,
};
use crate::terrain::watersheds::{PocketWater, PocketWatersHighPass, PocketWatersLowPass};

/// Landform or hydrology family that authored a stored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicFamily {
	Massif,
	Plateau,
	Canyon,
	Rolling,
	Valley,
	PocketWaterStamp,
	Watershed,
}

/// Occupancy band that produced the authored record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicBand {
	HighPass,
	LowPass,
}

/// Terrain vocabulary for an authored geographic source.
///
/// A massif stamp is an authored massif contribution, not proof that the
/// blended landscape contains one connected range. A streams graph is a bag of
/// corridors, not a river-connectivity graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeographicFeatureKind {
	Massif,
	Plateau,
	Canyon,
	Rolling,
	Valley,
	PocketWater,
	Lake,
	Bog,
	Stream,
	StreamsGraph,
}

/// Stable identity of one authored geographic source.
///
/// Distinguishes family, band, and the source cell. Several derived terrain or
/// hydro-complex cells may contain the same lake; they reference this identity
/// rather than acquiring names of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GeographicFeatureId {
	pub family: GeographicFamily,
	pub band: GeographicBand,
	pub source: Id,
}

/// Conservative descriptor of one stored geographic source.
///
/// `bounds` is the stamp or watershed leaf AABB in XZ. That is a first
/// approximation, not a connected-feature hull. `anchor` is a representative
/// point for language selection and label placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeographicFeature {
	pub id: GeographicFeatureId,
	pub revision: Version,
	pub kind: GeographicFeatureKind,
	pub bounds: Bounds2,
	pub anchor: Vec2,
}

/// Stored geographic sources whose leaf bounds overlap `region`; see
/// [`crate::terrain::TerrainStorage::geographic_features_overlapping`].
pub(crate) fn geographic_features_overlapping(
	storage: &HcsgStorage,
	region: Bounds2,
) -> Result<Vec<GeographicFeature>, Busy> {
	let mut out = Vec::new();
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Massif,
		GeographicBand::HighPass,
		GeographicFeatureKind::Massif,
		|stamp: &MassifHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Massif,
		GeographicBand::LowPass,
		GeographicFeatureKind::Massif,
		|stamp: &MassifLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Plateau,
		GeographicBand::HighPass,
		GeographicFeatureKind::Plateau,
		|stamp: &PlateauHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Plateau,
		GeographicBand::LowPass,
		GeographicFeatureKind::Plateau,
		|stamp: &PlateauLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Canyon,
		GeographicBand::HighPass,
		GeographicFeatureKind::Canyon,
		|stamp: &CanyonHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Canyon,
		GeographicBand::LowPass,
		GeographicFeatureKind::Canyon,
		|stamp: &CanyonLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Rolling,
		GeographicBand::HighPass,
		GeographicFeatureKind::Rolling,
		|stamp: &RollingHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Rolling,
		GeographicBand::LowPass,
		GeographicFeatureKind::Rolling,
		|stamp: &RollingLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Valley,
		GeographicBand::HighPass,
		GeographicFeatureKind::Valley,
		|stamp: &ValleyHighPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::Valley,
		GeographicBand::LowPass,
		GeographicFeatureKind::Valley,
		|stamp: &ValleyLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::PocketWaterStamp,
		GeographicBand::HighPass,
		GeographicFeatureKind::PocketWater,
		|stamp: &PocketWaterHighPassStampCell| {
			(!stamp.modulations.is_empty()).then_some(stamp.cell)
		},
	)?;
	push_stamp_features(
		&mut out,
		region,
		storage,
		GeographicFamily::PocketWaterStamp,
		GeographicBand::LowPass,
		GeographicFeatureKind::PocketWater,
		|stamp: &PocketWaterLowPassStampCell| (!stamp.modulations.is_empty()).then_some(stamp.cell),
	)?;
	push_watershed_features::<PocketWatersHighPass>(&mut out, region, storage)?;
	push_watershed_features::<PocketWatersLowPass>(&mut out, region, storage)?;
	Ok(out)
}

/// Latest membership change among the stores [`geographic_features_overlapping`]
/// reads. Writes to any other type, inside Durham or not, leave it unchanged.
pub(crate) fn geography_revision(storage: &HcsgStorage) -> Result<u64, Busy> {
	let mut latest = 0u64;
	for revision in [
		storage.try_membership_revision::<MassifHighPassStampCell>()?,
		storage.try_membership_revision::<MassifLowPassStampCell>()?,
		storage.try_membership_revision::<PlateauHighPassStampCell>()?,
		storage.try_membership_revision::<PlateauLowPassStampCell>()?,
		storage.try_membership_revision::<CanyonHighPassStampCell>()?,
		storage.try_membership_revision::<CanyonLowPassStampCell>()?,
		storage.try_membership_revision::<RollingHighPassStampCell>()?,
		storage.try_membership_revision::<RollingLowPassStampCell>()?,
		storage.try_membership_revision::<ValleyHighPassStampCell>()?,
		storage.try_membership_revision::<ValleyLowPassStampCell>()?,
		storage.try_membership_revision::<PocketWaterHighPassStampCell>()?,
		storage.try_membership_revision::<PocketWaterLowPassStampCell>()?,
		storage.try_membership_revision::<PocketWatersHighPass>()?,
		storage.try_membership_revision::<PocketWatersLowPass>()?,
	] {
		latest = latest.max(revision);
	}
	Ok(latest)
}

fn push_stamp_features<T: HcsgValue>(
	out: &mut Vec<GeographicFeature>,
	region: Bounds2,
	storage: &HcsgStorage,
	family: GeographicFamily,
	band: GeographicBand,
	kind: GeographicFeatureKind,
	cell_if_occupied: impl Fn(&T) -> Option<Aabb3d>,
) -> Result<(), Busy> {
	let query = bounds2_query_aabb(region);
	let ids = storage.try_overlapping::<T>(query)?;
	for id in ids {
		let Some(entry) = storage.try_entry::<T>(id)? else {
			continue;
		};
		let Some(cell) = cell_if_occupied(entry.value.as_ref()) else {
			continue;
		};
		let bounds = bounds2_from_aabb(cell);
		if !xz_overlaps(region, bounds) {
			continue;
		}
		out.push(GeographicFeature {
			id: GeographicFeatureId { family, band, source: id },
			revision: entry.version,
			kind,
			bounds,
			anchor: bounds.center(),
		});
	}
	Ok(())
}

fn push_watershed_features<T>(
	out: &mut Vec<GeographicFeature>,
	region: Bounds2,
	storage: &HcsgStorage,
) -> Result<(), Busy>
where
	T: AuthoredPocketWaters + HcsgValue,
{
	let query = bounds2_query_aabb(region);
	let ids = storage.try_overlapping::<T>(query)?;
	for id in ids {
		let Some(entry) = storage.try_entry::<T>(id)? else {
			continue;
		};
		let value = entry.value.as_ref();
		let authored = value.authored();
		let Some((kind, bounds, anchor)) = authored_geography(authored, value.cell()) else {
			continue;
		};
		if !xz_overlaps(region, bounds) {
			continue;
		};
		out.push(GeographicFeature {
			id: GeographicFeatureId {
				family: GeographicFamily::Watershed,
				band: value.band(),
				source: id,
			},
			revision: entry.version,
			kind,
			bounds,
			anchor,
		});
	}
	Ok(())
}

trait AuthoredPocketWaters {
	fn authored(&self) -> &PocketWater;
	fn cell(&self) -> Aabb3d;
	fn band(&self) -> GeographicBand;
}

impl AuthoredPocketWaters for PocketWatersHighPass {
	fn authored(&self) -> &PocketWater {
		&self.authored
	}

	fn cell(&self) -> Aabb3d {
		self.cell
	}

	fn band(&self) -> GeographicBand {
		GeographicBand::HighPass
	}
}

impl AuthoredPocketWaters for PocketWatersLowPass {
	fn authored(&self) -> &PocketWater {
		&self.authored
	}

	fn cell(&self) -> Aabb3d {
		self.cell
	}

	fn band(&self) -> GeographicBand {
		GeographicBand::LowPass
	}
}

fn authored_geography(
	authored: &PocketWater,
	_leaf: Aabb3d,
) -> Option<(GeographicFeatureKind, Bounds2, Vec2)> {
	match authored {
		PocketWater::Empty => None,
		PocketWater::Lake(lake) => Some((GeographicFeatureKind::Lake, lake.bounds, lake.center)),
		PocketWater::Bog(bog) => Some((GeographicFeatureKind::Bog, bog.bounds, bog.center)),
		PocketWater::Stream(stream) => {
			let anchor = stream
				.path
				.get(stream.path.len() / 2)
				.copied()
				.unwrap_or_else(|| stream.bounds.center());
			Some((GeographicFeatureKind::Stream, stream.bounds, anchor))
		}
		PocketWater::StreamsGraph(graph) => {
			Some((GeographicFeatureKind::StreamsGraph, graph.bounds, graph.bounds.center()))
		}
	}
}

fn bounds2_from_aabb(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

fn bounds2_query_aabb(region: Bounds2) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(region.min.x, f32::NEG_INFINITY, region.min.y),
		Vec3::new(region.max.x, f32::INFINITY, region.max.y),
	)
}

fn xz_overlaps(a: Bounds2, b: Bounds2) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

#[cfg(test)]
mod tests {
	use super::*;

	fn storage_busy(busy: Busy) -> anyhow::Error {
		anyhow::anyhow!("HcsgStorage busy: {busy:?}")
	}
	use crate::terrain::stamps::MassifHighPassStampCell;
	use crate::terrain::watersheds::{HydroComplexCell, PocketWatersHighPass, WatershedBandPass};
	use std::sync::Arc;
	use terrain_stamps::{CircleRegion, Region2D, RegionAffineModulation, StampModulation};
	use terrain_watersheds::{HydroComplex, Lake, LakeParams};

	fn cell(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Aabb3d {
		Aabb3d::from_min_max(
			bevy::math::Vec3::new(min_x, -1.0, min_z),
			bevy::math::Vec3::new(max_x, 1.0, max_z),
		)
	}

	fn dummy_modulation() -> StampModulation {
		StampModulation::Affine(RegionAffineModulation::new(
			Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 8.0 }),
			1.0,
			0.0,
			4.0,
			8.0,
		))
	}

	fn insert_stamp(store: &HcsgStorage, id: Id, stamp_cell: Aabb3d, occupied: bool) -> Version {
		store.publish(
			id,
			Arc::new(MassifHighPassStampCell {
				cell: stamp_cell,
				modulations: if occupied { vec![dummy_modulation()] } else { Vec::new() },
			}),
			stamp_cell,
		)
	}

	fn insert_lake(store: &HcsgStorage, id: Id, lake_cell: Aabb3d, lake: Lake) -> Version {
		store.publish(
			id,
			Arc::new(PocketWatersHighPass {
				cell: lake_cell,
				band: WatershedBandPass::High,
				authored: PocketWater::Lake(lake),
			}),
			lake_cell,
		)
	}

	#[test]
	fn empty_stamp_modulations_are_not_geographic_sources() -> anyhow::Result<()> {
		let store = HcsgStorage::default();
		let occupied = cell(0.0, 0.0, 100.0, 100.0);
		let empty = cell(200.0, 200.0, 300.0, 300.0);
		let version = insert_stamp(&store, Id::from_cell(occupied), occupied, true);
		insert_stamp(&store, Id::from_cell(empty), empty, false);

		let found =
			geographic_features_overlapping(&store, Bounds2::from_xz(-10.0, -10.0, 400.0, 400.0))
				.map_err(storage_busy)?;
		anyhow::ensure!(found.len() == 1, "expected one occupied stamp, got {}", found.len());
		anyhow::ensure!(found[0].kind == GeographicFeatureKind::Massif);
		anyhow::ensure!(found[0].id.family == GeographicFamily::Massif);
		anyhow::ensure!(found[0].id.band == GeographicBand::HighPass);
		anyhow::ensure!(found[0].id.source == Id::from_cell(occupied));
		anyhow::ensure!(found[0].revision == version);
		Ok(())
	}

	#[test]
	fn one_authored_lake_is_named_once_across_derived_hydro_cells() -> anyhow::Result<()> {
		let store = HcsgStorage::default();
		let lake_cell = cell(0.0, 0.0, 400.0, 400.0);
		let lake_bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let lake = Lake::from_bounds(lake_bounds, 7, LakeParams::default(), None)
			.ok_or_else(|| anyhow::anyhow!("authored lake"))?;
		let lake_id = Id::from_cell(lake_cell);
		let lake_version = insert_lake(&store, lake_id, lake_cell, lake);

		let left = cell(0.0, 0.0, 200.0, 400.0);
		let right = cell(200.0, 0.0, 400.0, 400.0);
		for derived in [left, right] {
			store.publish(
				Id::from_cell(derived),
				Arc::new(HydroComplexCell {
					cell: derived,
					complex: Arc::new(HydroComplex::new(
						Bounds2::from_xz(
							derived.min.x,
							derived.min.z,
							derived.max.x,
							derived.max.z,
						),
						1,
					)),
					marazion_leaves: Vec::new(),
				}),
				derived,
			);
		}

		let found =
			geographic_features_overlapping(&store, Bounds2::from_xz(-10.0, -10.0, 410.0, 410.0))
				.map_err(storage_busy)?;
		let lakes: Vec<_> = found
			.iter()
			.filter(|feature| feature.kind == GeographicFeatureKind::Lake)
			.collect();
		anyhow::ensure!(lakes.len() == 1, "derived hydro cells must not mint extra lakes");
		anyhow::ensure!(lakes[0].id.source == lake_id);
		anyhow::ensure!(lakes[0].id.family == GeographicFamily::Watershed);
		anyhow::ensure!(lakes[0].revision == lake_version);
		Ok(())
	}

	#[test]
	fn geography_revision_moves_only_for_source_stores() -> anyhow::Result<()> {
		struct Furniture;
		let store = HcsgStorage::default();
		let before = geography_revision(&store).map_err(storage_busy)?;
		let bounds = cell(0.0, 0.0, 100.0, 100.0);
		store.publish(Id::from_cell(bounds), Arc::new(Furniture), bounds);
		store.publish(
			Id::from_cell(bounds),
			Arc::new(HydroComplexCell {
				cell: bounds,
				complex: Arc::new(HydroComplex::new(Bounds2::from_xz(0.0, 0.0, 100.0, 100.0), 1)),
				marazion_leaves: Vec::new(),
			}),
			bounds,
		);
		anyhow::ensure!(
			geography_revision(&store).map_err(storage_busy)? == before,
			"non-source writes must not count"
		);
		insert_stamp(&store, Id::from_cell(bounds), bounds, true);
		anyhow::ensure!(
			geography_revision(&store).map_err(storage_busy)? > before,
			"a stamp write is a source change"
		);
		Ok(())
	}

	#[test]
	fn query_does_not_admit_or_mutate_storage() -> anyhow::Result<()> {
		let store = HcsgStorage::default();
		let before = geography_revision(&store).map_err(storage_busy)?;
		let count = geographic_features_overlapping(&store, Bounds2::from_xz(0.0, 0.0, 10.0, 10.0))
			.map_err(storage_busy)?
			.len();
		anyhow::ensure!(count == 0);
		anyhow::ensure!(geography_revision(&store).map_err(storage_busy)? == before);
		Ok(())
	}
}
