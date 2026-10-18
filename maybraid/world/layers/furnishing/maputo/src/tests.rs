//! [`DevelopmentSlots`] for a stored Les Halles development.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::FurnitureNode;
use buildings::{Confines, Fit};
use durham::Durham;
use furniture_usage_areas::expand_usages;
use lod::gen::Id;
use lod::hcsg::GenerationContext;
use lod::hcsg::HcsgStorage;
use procedural_common::NoiseParams;
use richmond::{Built, BuiltDevelopment, DevelopmentHosts};
use terrain_layer_model::OnTerrain;
use urbanization_developments::{MixedUseLesHallesDevelopment, PlacedBuilding};
use urbanization_layer_model::{UrbanSetting, Urbanization};

use crate::cell::world_slot;
use crate::hcsg::DevelopmentSlots;

type Ground = OnTerrain<Durham>;
type Urbanized = Urbanization<richmond::Richmond<Ground>>;

fn les_halles(yaw: f32) -> anyhow::Result<BuiltDevelopment> {
	let bounds = Aabb3d::from_min_max(Vec3::new(-18.0, 0.0, -18.0), Vec3::new(18.0, 10.0, 18.0));
	let (mut development, _) = PlacedBuilding::<MixedUseLesHallesDevelopment>::fit_to_confines(
		&Confines::from_bounds(bounds),
		NoiseParams { seed: 1337, ..NoiseParams::default() },
	)?;
	development.yaw = yaw;
	Ok(BuiltDevelopment::LesHalles(Box::new(development)))
}

fn expected_slots(development: &BuiltDevelopment) -> Vec<FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			out.push(world_slot(transform, node));
		}
		for node in expand_usages(host.furniture_usage_nodes()) {
			out.push(world_slot(transform, node));
		}
	}
	out
}

#[test]
fn development_slots_match_les_halles_world_slots() -> anyhow::Result<()> {
	let built = les_halles(0.4)?;
	let expected = expected_slots(&built);
	anyhow::ensure!(!expected.is_empty(), "Les Halles should emit High slots");
	let bounds = Aabb3d::from_min_max(Vec3::new(-40.0, -8.0, -40.0), Vec3::new(40.0, 24.0, 40.0));
	let id = Id::from_cell(bounds);
	let elsewhere =
		Aabb3d::from_min_max(Vec3::new(800.0, -8.0, 800.0), Vec3::new(880.0, 24.0, 880.0));
	let other = les_halles(0.0)?;

	let storage = HcsgStorage::default();
	{
		let setting = |id| UrbanSetting { id, arrival_radius: 8.0 };
		storage.publish(id, Arc::new(Built::<Ground>::new(built, setting(id), Vec3::ZERO)), bounds);
		let other_id = Id::from_cell(elsewhere);
		storage.publish(
			other_id,
			Arc::new(Built::<Ground>::new(other, setting(other_id), Vec3::ZERO)),
			elsewhere,
		);
	}

	let mut cx = GenerationContext::new(&storage);
	let slots = cx
		.get_or_generate::<DevelopmentSlots<Urbanized>>(id)
		.ok_or_else(|| anyhow::anyhow!("development slots"))?;
	anyhow::ensure!(slots.slots == expected, "world-space slots match the host list");

	let other_id = Id::from_cell(elsewhere);
	let other_slots = cx
		.get_or_generate::<DevelopmentSlots<Urbanized>>(other_id)
		.ok_or_else(|| anyhow::anyhow!("other development slots"))?;
	anyhow::ensure!(other_slots.slots != expected, "a different yaw is a different slot list");
	Ok(())
}
