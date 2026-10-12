//! Each built development's discoverable places, derived from Richmond's
//! [`Built`] value rather than read off spawned entities.

use std::marker::PhantomData;

use bevy::math::bounding::{Aabb3d, BoundingVolume};
use bevy::math::{Vec2, Vec3Swizzles};
use building_components::{BuildingComponents, LabelNode};
use chico::ForestGround;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use lod::LodSceneLevel;
use richmond::{
	Built, BuiltDevelopment, DevelopmentHost, DevelopmentHosts, DiscoverablePlace, Richmond,
	RichmondGround,
};
use urbanization_layer_model::Urbanization;

use crate::key::NameKey;

/// A ground Geneva names: Chico's groves grow on it and Richmond builds on
/// the ground under it.
pub trait LanguageGround: ForestGround {
	type Built: RichmondGround;
}

impl<G: RichmondGround> LanguageGround for Urbanization<Richmond<G>>
where
	Self: ForestGround,
{
	type Built = G;
}

/// One discoverable place in a built development.
#[derive(Clone, Debug, PartialEq)]
pub struct DevelopmentPlace {
	pub key: NameKey,
	pub place: DiscoverablePlace,
	pub xz: Vec2,
	/// For a room, the index of its building's place.
	pub building: Option<usize>,
}

/// The places of one development built on ground `W`, under the
/// development's id.
pub struct DevelopmentPlaces<W> {
	pub places: Vec<DevelopmentPlace>,
	_ground: PhantomData<fn() -> W>,
}

impl<W> DevelopmentPlaces<W> {
	pub fn new(places: Vec<DevelopmentPlace>) -> Self {
		Self { places, _ground: PhantomData }
	}

	/// Each host's place pin, then the usage-area rooms its High floors
	/// author, keyed as Richmond stamps them so the map joins them to its POIs.
	pub fn of(development: Id, built: &BuiltDevelopment) -> Self {
		let mut places = Vec::<DevelopmentPlace>::new();
		let mut push = |place: DevelopmentPlace| match places
			.iter()
			.position(|known| known.key == place.key)
		{
			Some(index) => index,
			None => {
				places.push(place);
				places.len() - 1
			}
		};
		for host in &built.hosts() {
			let transform = host.transform();
			let building = host.discoverable_place().map(|place| {
				let local = host.place_local_id();
				push(DevelopmentPlace {
					key: NameKey::Place { host: development, local },
					place: place.with_identity(development, local),
					xz: transform.transform_point(host.local_bounds().center().into()).xz(),
					building: None,
				})
			});
			for node in host_labels(host) {
				let Some(place) = DiscoverablePlace::from_label_node(&node) else {
					continue;
				};
				let local = local_place_id(&node);
				push(DevelopmentPlace {
					key: NameKey::Place { host: development, local },
					place: place.with_identity(development, local),
					xz: transform.transform_point(node.placement.translation).xz(),
					building,
				});
			}
		}
		Self::new(places)
	}
}

impl<W: LanguageGround> hcsg::GenerationScheme for DevelopmentPlaces<W> {
	lod::hcsg_index_scale!(crate::hcsg::PLACES_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<Built<W::Built>>(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (built, bounds) = cx.get_or_generate_stored::<Built<W::Built>>(id)?;
		Some((Self::of(id, &built.development), bounds))
	}
}

fn host_labels(host: &DevelopmentHost) -> Vec<LabelNode> {
	fn of(building: &impl BuildingComponents) -> Vec<LabelNode> {
		building.label_nodes_for_level(LodSceneLevel::High).flatten()
	}
	match host {
		DevelopmentHost::LesHallesStorey(building, _) => of(building.as_ref()),
		DevelopmentHost::LesHallesStairwell(building, _) => of(building.as_ref()),
		DevelopmentHost::LesHallesRoof(building, _) => of(building.as_ref()),
		DevelopmentHost::ShepherdsHouse(building, _) => of(building.as_ref()),
		DevelopmentHost::ShepherdsHut(building, _) => of(building.as_ref()),
		DevelopmentHost::OldCityMarketTerrace(building, _) => of(building.as_ref()),
		DevelopmentHost::RingFortCircularTower(building, _) => of(building.as_ref()),
		DevelopmentHost::RingFortTrazaloidTower(building, _) => of(building.as_ref()),
		DevelopmentHost::RingFortGalleryTerrace(building, _) => of(building.as_ref()),
		DevelopmentHost::RingFortGalleryColonnade(building, _) => of(building.as_ref()),
		DevelopmentHost::RingFortGalleryRoof(building, _) => of(building.as_ref()),
		DevelopmentHost::SingleHighrise(building, _) => of(building.as_ref()),
		DevelopmentHost::TempleSanctum(building, _) => of(building.as_ref()),
		DevelopmentHost::WizardsTower(building, _) => of(building.as_ref()),
		DevelopmentHost::SkybridgeHall(building, _) => of(building.as_ref()),
	}
}

/// Richmond's durable room id: an FNV hash of the authored usage-area pose.
fn local_place_id(node: &LabelNode) -> u32 {
	let translation = node.placement.translation;
	let extents = node.geometry.extents();
	let mut h = 2_166_131_261u32;
	for bits in [
		translation.x.to_bits(),
		translation.y.to_bits(),
		translation.z.to_bits(),
		extents.x.to_bits(),
		extents.y.to_bits(),
		extents.z.to_bits(),
		node.placement.yaw.to_bits(),
	] {
		h = h.wrapping_mul(16_777_619) ^ bits;
	}
	h
}
