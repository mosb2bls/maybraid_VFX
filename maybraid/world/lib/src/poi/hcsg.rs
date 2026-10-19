//! World POIs as [`PoiOrigin`] schemes over existing HCSG grids.
//!
//! Distant-tile journeying calls [`DistantTilePois::collect_matching_in_xz_tile`],
//! which resolves each family through `get_or_generate_in` — O(tile), not a
//! walk of every global POI.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, IVec2, Vec3};
use chico::ChicoGrove;
use geneva::{DevelopmentPlaces, LanguageGround};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{GenerationContext, HcsgStorage};
use mob_characters::{URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	collect_matching_in_xz_tile, hashed_poi_id, DistantTilePois, PoiEntry, PoiInterests, PoiOrigin,
	PoiSet,
};
use richmond::{Built, RichmondGround};

use super::place_kind;

const GROVE_POI_SALT: u64 = 0x6772_6f76_652d_706f;
const URBAN_POI_SALT: u64 = 0x7572_6261_6e2d_706f;
const INTERIOR_POI_SALT: u64 = 0x696e_7465_7269_6f72;

/// One vegetation destination per [`ChicoGrove`].
pub struct GrovePois;

impl PoiOrigin for GrovePois {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<ChicoGrove>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (grove, bounds) = cx.get_or_generate_stored::<ChicoGrove>(id)?;
		let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		let radius = ((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5)
			.clamp(1.0, 256.0);
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(GROVE_POI_SALT, grove.id()),
				kind: VEGETATION_POI,
				position: center,
				arrival_radius: radius,
				salience: 1.25,
				local: false,
				global: true,
			}],
			bounds,
		))
	}
}

/// One urban destination per filled [`Built`] development.
pub struct UrbanPois<G>(PhantomData<fn() -> G>);

impl<G: RichmondGround> PoiOrigin for UrbanPois<G> {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<Built<G>>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (built, bounds) = cx.get_or_generate_stored::<Built<G>>(id)?;
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(URBAN_POI_SALT, built.setting.id),
				kind: URBAN_POI,
				position: built.setting_at,
				arrival_radius: built.setting.arrival_radius,
				salience: 1.5,
				local: false,
				global: true,
			}],
			bounds,
		))
	}
}

/// Local discoverable places of one development, from [`DevelopmentPlaces`].
pub struct PlacePois<W>(PhantomData<fn() -> W>);

impl<W: LanguageGround> PoiOrigin for PlacePois<W> {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<DevelopmentPlaces<W>>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (places, bounds) = cx.get_or_generate_stored::<DevelopmentPlaces<W>>(id)?;
		let height = (bounds.min.y + bounds.max.y) * 0.5;
		let pois = places
			.places
			.iter()
			.map(|place| PoiEntry {
				id: hashed_poi_id(
					INTERIOR_POI_SALT ^ place.place.label.salt(),
					(place.place.host, place.place.local),
				),
				kind: place_kind(place.place.label),
				position: Vec3::new(place.xz.x, height, place.xz.y),
				arrival_radius: place.place.arrival_radius,
				salience: place.place.salience.unwrap_or(place.place.label.default_salience()),
				local: true,
				global: false,
			})
			.collect();
		Some((pois, bounds))
	}
}

/// World tile lookup: groves, urban settings, and places on the live ground.
pub struct WorldPoiTiles;

impl DistantTilePois for WorldPoiTiles {
	fn collect_matching_in_xz_tile(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	) {
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

		out.clear();
		let mut cx = GenerationContext::new(storage);
		let mut scratch = Vec::new();
		collect_matching_in_xz_tile::<GrovePois>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		collect_matching_in_xz_tile::<UrbanPois<Ground>>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		collect_matching_in_xz_tile::<PlacePois<Urban>>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		out.sort_unstable_by_key(|poi| poi.id);
		out.dedup_by_key(|poi| poi.id);
	}
}

/// Keeps world [`PoiSet`]s warm in the same neighborhoods that present groves
/// and developments, so distant-tile probes often hit already-published values.
pub fn register_world_poi_generation(app: &mut bevy::prelude::App) {
	use chico::GroveNeighborhood;
	use lod::hcsg::GenerationPlugin;
	use maybraid_game_mode_discover::InDiscovery;
	use richmond::DevelopmentNeighborhood;

	type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
	type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

	if !app.is_plugin_added::<GenerationPlugin<InDiscovery<GroveNeighborhood>, PoiSet<GrovePois>>>()
	{
		app.add_plugins((
			GenerationPlugin::<InDiscovery<GroveNeighborhood>, PoiSet<GrovePois>>::default(),
			GenerationPlugin::<InDiscovery<DevelopmentNeighborhood>, PoiSet<UrbanPois<Ground>>>::default(),
			GenerationPlugin::<InDiscovery<DevelopmentNeighborhood>, PoiSet<PlacePois<Urban>>>::default(),
		));
	}
}
