//! The world's layer stack satisfies every `finish` requirement headless.

use bevy::prelude::{
	App, AssetApp, AssetPlugin, Mesh, MinimalPlugins, StandardMaterial, Transform, Vec3,
};
use bevy::scene::ScenePlugin;
use bevy::state::app::StatesPlugin;
use durham::{
	Durham, NearStream, StreamRing, TerrainCellLayout, TerrainMeshAssets, TerrainStampConfigs,
	WatershedConfigs,
};
use lod::gen::Id;
use lod::hcsg::{HcsgRegions, HcsgRestartRequest, HcsgStorage};
use lod::LodViewer;
use maybraid_game_mode_discover::InDiscovery;
use maybraid_world::WorldLayersPlugin;
use terrain_layer_model::TerrainStreaming;

type NearRing = InDiscovery<StreamRing<NearStream>>;

fn assert_durham_roots_seeded(storage: &HcsgStorage) -> anyhow::Result<()> {
	let universal = Id::Universal;
	let rooted = |name: &str, present: bool| -> anyhow::Result<()> {
		anyhow::ensure!(present, "Durham {name} root");
		Ok(())
	};
	rooted(
		"layout",
		storage
			.try_entry::<TerrainCellLayout>(universal)
			.map_err(|_| anyhow::anyhow!("storage busy"))?
			.is_some(),
	)?;
	rooted(
		"stamp configs",
		storage
			.try_entry::<TerrainStampConfigs>(universal)
			.map_err(|_| anyhow::anyhow!("storage busy"))?
			.is_some(),
	)?;
	rooted(
		"watershed configs",
		storage
			.try_entry::<WatershedConfigs>(universal)
			.map_err(|_| anyhow::anyhow!("storage busy"))?
			.is_some(),
	)?;
	rooted(
		"terrain mesh assets",
		storage
			.try_entry::<TerrainMeshAssets>(universal)
			.map_err(|_| anyhow::anyhow!("storage busy"))?
			.is_some(),
	)?;
	Ok(())
}

fn near_regions_sent(app: &App) -> usize {
	app.world()
		.resource::<bevy::ecs::message::Messages<HcsgRegions<NearRing>>>()
		.iter_current_update_messages()
		.filter(|regions| !regions.boxes.is_empty())
		.count()
}

#[test]
fn discovery_seeds_its_session_and_streams_only_when_enabled() -> anyhow::Result<()> {
	let mut app = App::new();
	// Mob scenes run with the player stack, which this app does not install.
	app.set_error_handler(bevy::ecs::error::warn);
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin, ScenePlugin))
		.init_asset::<Mesh>()
		.init_asset::<StandardMaterial>()
		.init_asset::<bevy::world_serialization::WorldAsset>();
	app.add_plugins(WorldLayersPlugin);
	app.finish();
	app.cleanup();
	app.world_mut()
		.spawn((LodViewer, Transform::from_translation(Vec3::new(40.0, 0.0, 40.0))));
	app.insert_resource(TerrainStreaming::<Durham>::new(false));
	app.update();

	let storage = app.world().resource::<HcsgStorage>().clone();
	assert_durham_roots_seeded(&storage)?;
	anyhow::ensure!(near_regions_sent(&app) == 0, "the gate holds while streaming is off");

	app.world_mut().resource_mut::<HcsgRestartRequest>().request();
	app.update();
	assert_durham_roots_seeded(app.world().resource::<HcsgStorage>())?;

	app.insert_resource(TerrainStreaming::<Durham>::new(true));
	app.update();
	anyhow::ensure!(near_regions_sent(&app) == 1, "streaming sends the near ring");
	Ok(())
}
