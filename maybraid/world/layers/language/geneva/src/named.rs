//! Names as generated values: [`Named<S>`] holds the names of source `S`'s
//! value under the same id.
//!
//! A name is a pure function of the seed, its key, the English its source
//! value gives it, and the large tile under its anchor; a room speaks the
//! language of its building in the same value. So names generate on the
//! worker like any other value.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Rect, Vec2};
use chico::{ChicoForest, ChicoGrove, GrownGrove};
use durham::terrain::stamps::StampLeaf;
use durham::terrain::watersheds::{PocketWater, PocketWatersHighPass, PocketWatersLowPass};
use durham::terrain::{
	CanyonHighPassStampCell, CanyonLowPassStampCell, MassifHighPassStampCell,
	MassifLowPassStampCell, PlateauHighPassStampCell, PlateauLowPassStampCell,
	PocketWaterHighPassStampCell, PocketWaterLowPassStampCell, RollingHighPassStampCell,
	RollingLowPassStampCell, ValleyHighPassStampCell, ValleyLowPassStampCell,
};
use durham::{GeographicBand, GeographicFamily, GeographicFeatureId, GeographicFeatureKind};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use maybraid_language_core::lexicalizer::mix;
use procedural_common::Bounds2;
use urbanization_cells::SelectedUrbanization;

use crate::bundle::LanguageBundle;
use crate::english::{
	named_forest_english, named_geographic_english, named_grove_english, named_place_english,
	named_region_english, named_urban_english,
};
use crate::hcsg::LanguageWorldSeed;
use crate::key::{name_key_salt, NameKey};
use crate::name::PlaceName;
use crate::places::{DevelopmentPlaces, LanguageGround};
use crate::tiles::{
	large_tile_aabb, large_tile_index, large_tile_original_ids_overlapping, LargeTile,
};

/// A place's footprint on the map, around its anchor.
const PLACE_EXTENT: f32 = 12.0;

/// Something a source value holds that Geneva names.
#[derive(Clone, Debug, PartialEq)]
pub struct Nameable {
	pub key: NameKey,
	pub xz: Vec2,
	pub extent: Rect,
	pub english: Vec<String>,
	pub speaks: Speaks,
}

/// Which language names a [`Nameable`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaks {
	/// The small-tile language under the anchor, picked by the key.
	Here,
	/// Large tile `(ix, iz)`'s regional language.
	Region { ix: i32, iz: i32 },
	/// The language of an earlier nameable in the same value: a room
	/// speaks its building's.
	As(usize),
}

/// One name, anchored where its source stands.
#[derive(Clone, Debug, PartialEq)]
pub struct NameEntry {
	pub key: NameKey,
	pub name: PlaceName,
	pub xz: Vec2,
	pub extent: Rect,
}

/// The names of source `S`'s value, under its id and bounds.
pub struct Named<S> {
	pub names: Vec<NameEntry>,
	_source: PhantomData<fn() -> S>,
}

/// A value Geneva names: which ids originate in a region, and what each holds.
pub trait NameSource: Send + Sync + 'static {
	/// Spatial index scale for [`Named<S>`].
	const INDEX_SCALE: DVec3;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

	/// What value `id` holds to name, and its bounds; `None` where it does not exist.
	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)>;
}

impl<S: NameSource> hcsg::GenerationScheme for Named<S> {
	const INDEX_SCALE: DVec3 = S::INDEX_SCALE;
	const RETENTION_MARGIN: DVec3 = S::INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		S::original_ids_for(cx, region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (nameables, bounds) = S::nameables(cx, id)?;
		let seed = cx.get::<LanguageWorldSeed>(Id::Universal)?.0;
		Some((Self { names: translate(cx, seed, &nameables), _source: PhantomData }, bounds))
	}
}

/// Names each nameable that has a language and a surface.
fn translate(cx: &mut GenerationContext, seed: u64, nameables: &[Nameable]) -> Vec<NameEntry> {
	let mut tiles = HashMap::<(i32, i32), Option<Arc<LargeTile>>>::new();
	let mut tile = |cx: &mut GenerationContext, ix: i32, iz: i32| {
		tiles
			.entry((ix, iz))
			.or_insert_with(|| cx.get_or_generate::<LargeTile>(LargeTile::id(ix, iz)))
			.clone()
	};
	let mut spoken = Vec::<Option<LanguageBundle>>::with_capacity(nameables.len());
	let mut names = Vec::new();
	for nameable in nameables {
		let english = &nameable.english;
		let named = match nameable.speaks {
			Speaks::Region { ix, iz } => tile(cx, ix, iz).and_then(|tile| {
				let bundle = *tile.region_language(seed)?;
				let name =
					PlaceName::translate(&bundle, english, mix(seed ^ 0x51A7 ^ mix(ix as u64)));
				Some((bundle, name))
			}),
			Speaks::As(index) if spoken.get(index).is_some_and(Option::is_some) => {
				spoken[index].map(|bundle| (bundle, PlaceName::translate_all(&bundle, english)))
			}
			Speaks::Here | Speaks::As(_) => {
				let at = nameable.xz;
				tile(cx, large_tile_index(at.x), large_tile_index(at.y)).and_then(|tile| {
					let bundle = *tile.language_at(at, name_key_salt(nameable.key))?;
					Some((bundle, PlaceName::translate_all(&bundle, english)))
				})
			}
		};
		let named = named.filter(|(_, name)| !name.surface.is_empty());
		if let Some((_, name)) = &named {
			names.push(NameEntry {
				key: nameable.key,
				name: name.clone(),
				xz: nameable.xz,
				extent: nameable.extent,
			});
		}
		spoken.push(named.map(|(bundle, _)| bundle));
	}
	names
}

fn center(bounds: Aabb3d) -> Vec2 {
	Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5)
}

fn extent(bounds: Aabb3d) -> Rect {
	Rect::from_corners(Vec2::new(bounds.min.x, bounds.min.z), Vec2::new(bounds.max.x, bounds.max.z))
}

/// One nameable standing over `bounds`, in the language under its center.
fn feature(key: NameKey, bounds: Aabb3d, english: Vec<String>) -> Nameable {
	Nameable { key, xz: center(bounds), extent: extent(bounds), english, speaks: Speaks::Here }
}

/// Each large tile, named in its regional language.
pub struct Regions;

impl NameSource for Regions {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		large_tile_original_ids_overlapping(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (ix, iz) = LargeTile::index_of(id)?;
		let seed = cx.get::<LanguageWorldSeed>(Id::Universal)?.0;
		let bounds = large_tile_aabb(ix, iz);
		let region = Nameable {
			speaks: Speaks::Region { ix, iz },
			..feature(NameKey::Region { ix, iz }, bounds, named_region_english(seed, ix, iz))
		};
		Some((vec![region], bounds))
	}
}

/// Chico's forests, by their layering and grove kinds.
pub struct Forests;

impl NameSource for Forests {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<ChicoForest>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (forest, bounds) = cx.get_or_generate_stored::<ChicoForest>(id)?;
		let layers = &forest.layers;
		let kinds = [layers.tufts, layers.understory, layers.lower_canopy, layers.upper_canopy];
		let key = NameKey::Forest(id);
		let english =
			named_forest_english(layers.layering, kinds.into_iter().flatten(), name_key_salt(key));
		Some((vec![feature(key, bounds, english)], bounds))
	}
}

/// The groves that grew on ground `W`.
pub struct Groves<W>(PhantomData<fn() -> W>);

impl<W: LanguageGround> NameSource for Groves<W> {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<ChicoGrove>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (grove, bounds) = cx.get_or_generate_stored::<ChicoGrove>(id)?;
		if cx.get_or_generate::<GrownGrove<W>>(id).is_none() {
			return Some((Vec::new(), bounds));
		}
		let key = NameKey::Grove(id);
		let kinds = grove.recipes.iter().map(|recipe| recipe.kind);
		let english = named_grove_english(kinds, name_key_salt(key));
		Some((vec![feature(key, bounds, english)], bounds))
	}
}

/// Urbanization cells and their development leaves.
pub struct Urban;

impl NameSource for Urban {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<SelectedUrbanization>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (cell, bounds) = cx.get_or_generate_stored::<SelectedUrbanization>(id)?;
		let key = NameKey::Urban(id);
		let mut out =
			vec![feature(key, bounds, named_urban_english(cell.kind, None, name_key_salt(key)))];
		for leaf in &cell.leaves {
			let key = NameKey::UrbanLeaf(leaf.id());
			let english = named_urban_english(cell.kind, Some(leaf.kind), name_key_salt(key));
			out.push(feature(key, leaf.bounds, english));
		}
		Some((out, bounds))
	}
}

/// Richmond's places built on ground `W`: each building, and the rooms in
/// it, which speak their building's language.
pub struct Places<W>(PhantomData<fn() -> W>);

impl<W: LanguageGround> NameSource for Places<W> {
	const INDEX_SCALE: DVec3 = crate::hcsg::PLACES_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<DevelopmentPlaces<W>>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (places, bounds) = cx.get_or_generate_stored::<DevelopmentPlaces<W>>(id)?;
		let nameables = places
			.places
			.iter()
			.map(|place| Nameable {
				key: place.key,
				xz: place.xz,
				extent: Rect::from_center_size(place.xz, Vec2::splat(PLACE_EXTENT)),
				english: named_place_english(place.place.label, name_key_salt(place.key)),
				speaks: place.building.map_or(Speaks::Here, Speaks::As),
			})
			.collect();
		Some((nameables, bounds))
	}
}

/// A Durham stamp leaf Geneva names, and the geography it reads as.
pub trait GeographicStamp: StampLeaf + hcsg::GenerationScheme {
	const FAMILY: GeographicFamily;
	const BAND: GeographicBand;
	const KIND: GeographicFeatureKind;
}

macro_rules! geographic_stamp {
	($($Stamp:ty => $family:ident, $band:ident, $kind:ident;)+) => {
		$(
			impl GeographicStamp for $Stamp {
				const FAMILY: GeographicFamily = GeographicFamily::$family;
				const BAND: GeographicBand = GeographicBand::$band;
				const KIND: GeographicFeatureKind = GeographicFeatureKind::$kind;
			}
		)+
	};
}

geographic_stamp! {
	MassifHighPassStampCell => Massif, HighPass, Massif;
	MassifLowPassStampCell => Massif, LowPass, Massif;
	PlateauHighPassStampCell => Plateau, HighPass, Plateau;
	PlateauLowPassStampCell => Plateau, LowPass, Plateau;
	CanyonHighPassStampCell => Canyon, HighPass, Canyon;
	CanyonLowPassStampCell => Canyon, LowPass, Canyon;
	RollingHighPassStampCell => Rolling, HighPass, Rolling;
	RollingLowPassStampCell => Rolling, LowPass, Rolling;
	ValleyHighPassStampCell => Valley, HighPass, Valley;
	ValleyLowPassStampCell => Valley, LowPass, Valley;
	PocketWaterHighPassStampCell => PocketWaterStamp, HighPass, PocketWater;
	PocketWaterLowPassStampCell => PocketWaterStamp, LowPass, PocketWater;
}

/// An occupied Durham stamp leaf, named over its cell; an empty leaf names nothing.
pub struct Stamp<T>(PhantomData<fn() -> T>);

impl<T: GeographicStamp> NameSource for Stamp<T> {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<T>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (stamp, bounds) = cx.get_or_generate_stored::<T>(id)?;
		if stamp.modulations().is_empty() {
			return Some((Vec::new(), bounds));
		}
		let cell = stamp.cell();
		let source = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
		let id = GeographicFeatureId { family: T::FAMILY, band: T::BAND, source: id };
		Some((vec![geographic(id, T::KIND, source)], bounds))
	}
}

/// Durham's authored pocket waters for one band.
pub trait AuthoredWaters: hcsg::GenerationScheme {
	const BAND: GeographicBand;
	fn authored(&self) -> &PocketWater;
}

impl AuthoredWaters for PocketWatersHighPass {
	const BAND: GeographicBand = GeographicBand::HighPass;
	fn authored(&self) -> &PocketWater {
		&self.authored
	}
}

impl AuthoredWaters for PocketWatersLowPass {
	const BAND: GeographicBand = GeographicBand::LowPass;
	fn authored(&self) -> &PocketWater {
		&self.authored
	}
}

/// An authored lake, bog or stream, named once across the cells it feeds.
pub struct Waters<T>(PhantomData<fn() -> T>);

impl<T: AuthoredWaters> NameSource for Waters<T> {
	const INDEX_SCALE: DVec3 = crate::hcsg::TILE_INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<T>(region)
	}

	fn nameables(cx: &mut GenerationContext, id: Id) -> Option<(Vec<Nameable>, Aabb3d)> {
		let (waters, bounds) = cx.get_or_generate_stored::<T>(id)?;
		let (kind, source) = match waters.authored() {
			PocketWater::Empty => return Some((Vec::new(), bounds)),
			PocketWater::Lake(lake) => (GeographicFeatureKind::Lake, lake.bounds),
			PocketWater::Bog(bog) => (GeographicFeatureKind::Bog, bog.bounds),
			PocketWater::Stream(stream) => (GeographicFeatureKind::Stream, stream.bounds),
			PocketWater::StreamsGraph(graph) => (GeographicFeatureKind::StreamsGraph, graph.bounds),
		};
		let id =
			GeographicFeatureId { family: GeographicFamily::Watershed, band: T::BAND, source: id };
		Some((vec![geographic(id, kind, source)], bounds))
	}
}

fn geographic(id: GeographicFeatureId, kind: GeographicFeatureKind, source: Bounds2) -> Nameable {
	let key = NameKey::Geographic(id);
	let english = named_geographic_english(kind, name_key_salt(key));
	let extent = Rect::from_corners(
		Vec2::new(source.min.x, source.min.y),
		Vec2::new(source.max.x, source.max.y),
	);
	Nameable { key, xz: extent.center(), extent, english, speaks: Speaks::Here }
}
