use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use chico::ForestGroveKind;
use durham::terrain::watersheds::{PocketWater, PocketWatersHighPass};
use durham::terrain::MassifHighPassStampCell;
use durham::{
	Durham, GeographicBand, GeographicFamily, GeographicFeatureId, GeographicFeatureKind,
	WatershedBandPass,
};
use lod::gen::Id;
use lod::hcsg::universal_bounds;
use lod::hcsg::{GenerationContext, HcsgStorage};
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, DiscoverablePlaceLabel, Richmond};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

use crate::english::{
	compose_english, geographic_terms, grove_kind_terms, named_grove_english, named_region_english,
	with_color_name, PLACE_COLORS,
};
use crate::hcsg::LanguageWorldSeed;
use crate::key::{name_key_salt, NameKey};
use crate::name::{canonicalize_terms, pick_terms, PlaceName};
use crate::named::{NameEntry, NameSource, Named, Places, Regions, Stamp, Waters};
use crate::places::{DevelopmentPlace, DevelopmentPlaces};
use crate::tiles::{large_tiles_overlapping, LargeTile, LARGE_TILE};

type Urban = Urbanization<Richmond<OnTerrain<Durham>>>;

const SEED: u64 = 0xC0DE_F00D_A11A;

fn feature_aabb(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(min_x, -1.0, min_z), Vec3::new(max_x, 1.0, max_z))
}

fn origin_cell(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Id {
	Id::from_cell(feature_aabb(min_x, min_z, max_x, max_z))
}

/// A storage seeded with the language root, as a session starts.
fn session() -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.seed(LanguageWorldSeed(SEED), universal_bounds());
	storage
}

/// The names of source `S`'s value `id`, generated in `storage`.
fn names<S: NameSource>(storage: &HcsgStorage, id: Id) -> anyhow::Result<Vec<NameEntry>> {
	let named = GenerationContext::new(storage)
		.get_or_generate::<Named<S>>(id)
		.ok_or_else(|| anyhow::anyhow!("no names for {id:?}"))?;
	Ok(named.names.clone())
}

#[test]
fn explicit_vocab_does_not_split_debug_idents() -> anyhow::Result<()> {
	let riparian = grove_kind_terms(ForestGroveKind::RiparianMix);
	anyhow::ensure!(riparian.contains(&"riparian".to_owned()));
	anyhow::ensure!(riparian
		.iter()
		.any(|word| ["grove", "stand", "gallery", "woods"].contains(&word.as_str())));
	let oak = grove_kind_terms(ForestGroveKind::RollingOaks);
	anyhow::ensure!(oak.contains(&"oak".to_owned()));
	anyhow::ensure!(oak
		.iter()
		.any(|word| ["grove", "stand", "copse", "woods"].contains(&word.as_str())));
	let stream = geographic_terms(GeographicFeatureKind::StreamsGraph);
	anyhow::ensure!(stream.contains(&"stream".to_owned()));
	let massif = geographic_terms(GeographicFeatureKind::Massif);
	anyhow::ensure!(
		massif.contains(&"massif".to_owned()) && massif.contains(&"mountain".to_owned())
	);
	Ok(())
}

#[test]
fn grove_english_keeps_a_noun_head() -> anyhow::Result<()> {
	let phrase = named_grove_english([ForestGroveKind::RiparianMix], 7);
	anyhow::ensure!(phrase.first().is_some_and(|word| PLACE_COLORS.contains(&word.as_str())));
	anyhow::ensure!(
		phrase
			.last()
			.is_some_and(|word| ["grove", "stand", "gallery", "woods"].contains(&word.as_str())),
		"expected a grove noun, got {phrase:?}"
	);
	anyhow::ensure!(phrase.len() >= 2);
	let rolling = compose_english(&["rolling"], &["hills", "downs"], 3, false);
	anyhow::ensure!(
		rolling.last() == Some(&"hills".to_owned()) || rolling.last() == Some(&"downs".to_owned())
	);
	Ok(())
}

#[test]
fn color_names_vary_by_seed_and_keep_the_kind() -> anyhow::Result<()> {
	let first = with_color_name(vec!["bush".to_owned()], 1);
	anyhow::ensure!(first.len() == 2 && first[1] == "bush");
	anyhow::ensure!(PLACE_COLORS.contains(&first[0].as_str()));
	let colors: std::collections::HashSet<_> = (0..48)
		.map(|seed| with_color_name(vec!["bush".to_owned()], seed)[0].clone())
		.collect();
	anyhow::ensure!(colors.len() > 4, "expected several colors across seeds, got {colors:?}");
	Ok(())
}

#[test]
fn shuffled_terms_canonicalize_before_seeded_pick() -> anyhow::Result<()> {
	let a = ["taiga".to_owned(), "orchard".to_owned(), "river".to_owned()];
	let b = ["river".to_owned(), "orchard".to_owned(), "taiga".to_owned()];
	anyhow::ensure!(canonicalize_terms(&a) == canonicalize_terms(&b));
	anyhow::ensure!(pick_terms(&a, 9) == pick_terms(&b, 9));
	Ok(())
}

#[test]
fn large_tiles_compose_three_to_twelve_languages() -> anyhow::Result<()> {
	for iz in -2..=2 {
		for ix in -2..=2 {
			let tile = LargeTile::generate(SEED, ix, iz);
			anyhow::ensure!((3..=12).contains(&tile.languages.len()));
			anyhow::ensure!(!tile.small.is_empty());
			for small in &tile.small {
				anyhow::ensure!(!small.language_ids.is_empty());
				anyhow::ensure!(small
					.language_ids
					.iter()
					.all(|id| (*id as usize) < tile.languages.len()));
			}
		}
	}
	Ok(())
}

#[test]
fn neighboring_large_tiles_share_grammar_more_often_than_a_fresh_draw() -> anyhow::Result<()> {
	let a = LargeTile::generate(SEED, 0, 0);
	let neighbor = LargeTile::generate(SEED, 1, 0);
	let far = LargeTile::generate(SEED, 8, 8);
	let same_as_neighbor = a
		.languages
		.iter()
		.zip(neighbor.languages.iter())
		.filter(|(left, right)| left.grammar == right.grammar)
		.count();
	let same_as_far = a
		.languages
		.iter()
		.zip(far.languages.iter())
		.filter(|(left, right)| left.grammar == right.grammar)
		.count();
	anyhow::ensure!(
		same_as_neighbor >= same_as_far,
		"neighbor shares {same_as_neighbor} grammars, far shares {same_as_far}"
	);
	Ok(())
}

#[test]
fn tiles_and_names_are_deterministic() -> anyhow::Result<()> {
	let first = LargeTile::generate(SEED, 2, -1);
	let second = LargeTile::generate(SEED, 2, -1);
	anyhow::ensure!(first == second);

	let english = vec!["orchard".to_owned(), "hill".to_owned()];
	let name_a = PlaceName::translate(&first.languages[0], &english, 9);
	let name_b = PlaceName::translate(&second.languages[0], &english, 9);
	anyhow::ensure!(name_a == name_b);
	anyhow::ensure!(!name_a.surface.is_empty());
	Ok(())
}

#[test]
fn small_tiles_stay_inside_the_large_tile() -> anyhow::Result<()> {
	for (ix, iz) in [(0, 0), (1, -2), (-3, 4)] {
		let tile = LargeTile::generate(SEED, ix, iz);
		let parent = tile.bounds();
		anyhow::ensure!((9..=25).contains(&tile.small.len()));
		for small in &tile.small {
			anyhow::ensure!(small.bounds.min[0] >= parent.min[0] - 1e-3);
			anyhow::ensure!(small.bounds.min[1] >= parent.min[1] - 1e-3);
			anyhow::ensure!(small.bounds.max[0] <= parent.max[0] + 1e-3);
			anyhow::ensure!(small.bounds.max[1] <= parent.max[1] + 1e-3);
		}
	}
	Ok(())
}

#[test]
fn language_subset_uses_feature_key_not_lowest_id() -> anyhow::Result<()> {
	let tile = LargeTile::generate(SEED, 0, 0);
	let small = tile
		.small
		.iter()
		.find(|leaf| leaf.language_ids.len() > 1)
		.ok_or_else(|| anyhow::anyhow!("subset with more than one language"))?;
	let center = Vec2::new(
		(small.bounds.min[0] + small.bounds.max[0]) * 0.5,
		(small.bounds.min[1] + small.bounds.max[1]) * 0.5,
	);
	let first = tile
		.language(*small.language_ids.first().ok_or_else(|| anyhow::anyhow!("id"))?)
		.ok_or_else(|| anyhow::anyhow!("first language"))?
		.seed;
	let mut saw_other = false;
	for salt in 0..64u64 {
		let bundle = tile
			.language_at(center, salt.wrapping_mul(0x9E37))
			.ok_or_else(|| anyhow::anyhow!("bundle"))?;
		if bundle.seed != first {
			saw_other = true;
			break;
		}
	}
	anyhow::ensure!(saw_other, "subset pick must not always use the lowest language id");
	Ok(())
}

fn massif(cell: Aabb3d, occupied: bool) -> MassifHighPassStampCell {
	use terrain_stamps::{CircleRegion, Region2D, RegionAffineModulation, StampModulation};
	let modulation = StampModulation::Affine(RegionAffineModulation::new(
		Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 8.0 }),
		1.0,
		0.0,
		4.0,
		8.0,
	));
	MassifHighPassStampCell {
		cell,
		modulations: if occupied { vec![modulation] } else { Vec::new() },
	}
}

fn publish_massif(storage: &HcsgStorage, cell: Aabb3d, occupied: bool) -> Id {
	let id = Id::from_cell(cell);
	storage.publish(id, Arc::new(massif(cell, occupied)), cell);
	id
}

#[test]
fn region_names_come_from_seed_and_tile_not_streamed_features() -> anyhow::Result<()> {
	let bare = session();
	let streamed = session();
	publish_massif(&streamed, feature_aabb(100.0, 100.0, 200.0, 200.0), true);
	let tile = LargeTile::id(0, 0);
	let first = names::<Regions>(&bare, tile)?;
	anyhow::ensure!(first == names::<Regions>(&streamed, tile)?, "features must not move regions");

	let [region] = first.as_slice() else {
		anyhow::bail!("one name per tile, got {first:?}");
	};
	anyhow::ensure!(region.key == NameKey::Region { ix: 0, iz: 0 });
	anyhow::ensure!(!named_region_english(SEED, 0, 0).is_empty());
	anyhow::ensure!(!region.name.surface.is_empty());
	anyhow::ensure!((region.xz - Vec2::splat(LARGE_TILE * 0.5)).length() < 1e-3);
	anyhow::ensure!((region.extent.min - Vec2::ZERO).length() < 1e-3);
	anyhow::ensure!((region.extent.max - Vec2::splat(LARGE_TILE)).length() < 1e-3);
	Ok(())
}

#[test]
fn geography_names_occupied_stamps_and_each_authored_lake_once() -> anyhow::Result<()> {
	let storage = session();
	let occupied = publish_massif(&storage, feature_aabb(0.0, 0.0, 100.0, 100.0), true);
	let empty = publish_massif(&storage, feature_aabb(200.0, 200.0, 300.0, 300.0), false);
	let lake_cell = feature_aabb(-300.0, -300.0, -100.0, -100.0);
	let lake = terrain_watersheds::Lake::from_bounds(
		Bounds2::from_xz(-260.0, -260.0, -140.0, -140.0),
		7,
		terrain_watersheds::LakeParams::default(),
		None,
	)
	.ok_or_else(|| anyhow::anyhow!("authored lake"))?;
	let lake_id = Id::from_cell(lake_cell);
	storage.publish(
		lake_id,
		Arc::new(PocketWatersHighPass {
			cell: lake_cell,
			band: WatershedBandPass::High,
			authored: PocketWater::Lake(lake),
		}),
		lake_cell,
	);

	let stamp = names::<Stamp<MassifHighPassStampCell>>(&storage, occupied)?;
	let [stamp] = stamp.as_slice() else {
		anyhow::bail!("an occupied stamp has one name, got {stamp:?}");
	};
	anyhow::ensure!(
		stamp.key
			== NameKey::Geographic(GeographicFeatureId {
				family: GeographicFamily::Massif,
				band: GeographicBand::HighPass,
				source: occupied,
			})
	);
	anyhow::ensure!((stamp.xz - Vec2::splat(50.0)).length() < 1e-3);
	anyhow::ensure!((stamp.extent.min - Vec2::ZERO).length() < 1e-3);
	anyhow::ensure!((stamp.extent.max - Vec2::splat(100.0)).length() < 1e-3);
	anyhow::ensure!(names::<Stamp<MassifHighPassStampCell>>(&storage, empty)?.is_empty());

	let lake = names::<Waters<PocketWatersHighPass>>(&storage, lake_id)?;
	let [lake] = lake.as_slice() else {
		anyhow::bail!("an authored lake has one name, got {lake:?}");
	};
	anyhow::ensure!(matches!(
		lake.key,
		NameKey::Geographic(GeographicFeatureId { family: GeographicFamily::Watershed, .. })
	));
	anyhow::ensure!(lake.extent.min == Vec2::splat(-260.0), "authored bounds, not the cell");
	anyhow::ensure!(lake.extent.max == Vec2::splat(-140.0));
	Ok(())
}

#[test]
fn a_name_does_not_depend_on_what_was_named_first() -> anyhow::Result<()> {
	let a = feature_aabb(0.0, 0.0, 100.0, 100.0);
	let b = feature_aabb(200.0, 0.0, 300.0, 100.0);
	let first = session();
	let second = session();
	let (a_id, b_id) = (publish_massif(&first, a, true), publish_massif(&first, b, true));
	publish_massif(&second, a, true);
	publish_massif(&second, b, true);

	let a_first = names::<Stamp<MassifHighPassStampCell>>(&first, a_id)?;
	let b_first = names::<Stamp<MassifHighPassStampCell>>(&first, b_id)?;
	let b_second = names::<Stamp<MassifHighPassStampCell>>(&second, b_id)?;
	let a_second = names::<Stamp<MassifHighPassStampCell>>(&second, a_id)?;
	anyhow::ensure!(a_first == a_second && b_first == b_second);
	Ok(())
}

#[test]
fn rooms_speak_their_buildings_language_wherever_they_stand() -> anyhow::Result<()> {
	let storage = session();
	let development = origin_cell(0.0, 0.0, 50.0, 50.0);
	let building = NameKey::Place { host: development, local: 4 };
	let room = NameKey::Place { host: development, local: 99 };
	let loose = NameKey::Place { host: development, local: 100 };
	let place = |label, key: NameKey, xz: Vec2, building| {
		let NameKey::Place { local, .. } = key else {
			unreachable!("place keys only");
		};
		DevelopmentPlace {
			key,
			place: DiscoverablePlace::high(label, 3.0, 1.1).with_identity(development, local),
			xz,
			building,
		}
	};
	let across = Vec2::new(LARGE_TILE + 12.0, 18.0);
	let places = DevelopmentPlaces::<Urban>::new(vec![
		place(DiscoverablePlaceLabel::House, building, Vec2::new(12.0, 18.0), None),
		place(DiscoverablePlaceLabel::Lounge, room, across, Some(0)),
		place(DiscoverablePlaceLabel::Lounge, loose, across, None),
	]);
	storage.publish(development, Arc::new(places), feature_aabb(0.0, 0.0, 50.0, 50.0));

	let named = names::<Places<Urban>>(&storage, development)?;
	let name = |key: NameKey| {
		named
			.iter()
			.find(|entry| entry.key == key)
			.ok_or_else(|| anyhow::anyhow!("{key:?}"))
	};
	let (building, room, loose) = (name(building)?, name(room)?, name(loose)?);
	anyhow::ensure!(room.name.language_seed == building.name.language_seed, "the room inherits");
	anyhow::ensure!(room.xz == across, "a room anchors where it stands");

	let tile = LargeTile::generate(SEED, 1, 0);
	let here = tile
		.language_at(across, name_key_salt(loose.key))
		.ok_or_else(|| anyhow::anyhow!("language"))?;
	anyhow::ensure!(loose.name.language_seed == here.seed, "a loose room speaks its tile's");
	Ok(())
}

#[test]
fn unrelated_storage_writes_do_not_move_the_overlay_revision() -> anyhow::Result<()> {
	struct Furniture;
	let storage = session();
	let revision = |storage: &HcsgStorage| {
		crate::present::revision::<Urban>(storage).map_err(|_| anyhow::anyhow!("busy"))
	};
	let before = revision(&storage)?;
	let bounds = feature_aabb(0.0, 0.0, 160.0, 160.0);
	storage.publish(Id::from_cell(bounds), Arc::new(Furniture), bounds);
	anyhow::ensure!(revision(&storage)? == before, "furniture writes must not re-present");
	let stamp = publish_massif(&storage, bounds, true);
	anyhow::ensure!(revision(&storage)? == before, "a source write alone names nothing");
	names::<Stamp<MassifHighPassStampCell>>(&storage, stamp)?;
	anyhow::ensure!(revision(&storage)? > before, "a new name re-presents");
	Ok(())
}

#[test]
fn far_tile_lines_bound_the_overlap_exactly() {
	let snapped = feature_aabb(-2.0 * LARGE_TILE, 0.0, 2.0 * LARGE_TILE, LARGE_TILE);
	let tiles: Vec<_> = large_tiles_overlapping(snapped).collect();
	assert_eq!(tiles, [(-2, 0), (-1, 0), (0, 0), (1, 0)]);
}
