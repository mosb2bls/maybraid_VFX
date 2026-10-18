use std::path::{Path, PathBuf};

use bevy::prelude::*;
use world_materials_playground::{
	PendingStartupCommand, PlaygroundCommand, WorldMaterialsPlaygroundPlugin,
};

fn assets_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../assets")
}

fn main() {
	let startup = PlaygroundCommand::parse_startup_command().unwrap_or_else(|error| {
		eprintln!("{error}");
		std::process::exit(2);
	});
	if startup.is_some() {
		println!("Startup command from argv (same as in-game / text).");
	} else {
		println!(
			"World materials playground — press / for commands (`show hex`, `show pulse`, `shoot`)."
		);
	}

	let assets_path = assets_root();
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Maybraid World Materials".into(),
						resolution: (1280, 720).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin { file_path: assets_path.to_string_lossy().into(), ..default() }),
		)
		.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
		.insert_resource(PendingStartupCommand(startup))
		.add_plugins(WorldMaterialsPlaygroundPlugin)
		.run();
}
