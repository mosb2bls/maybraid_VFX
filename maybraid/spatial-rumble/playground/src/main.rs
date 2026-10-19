use bevy::prelude::*;
use spatial_rumble_playground::SpatialRumblePlaygroundPlugin;

fn main() {
	let demo = spatial_rumble_playground::PlaygroundDemo::from_args(std::env::args());
	println!("Spatial rumble playground — 1 Bray, 2 landing, 3 stack, 4 far, P pause.");
	if demo.enabled {
		println!("Demo: repeating stacked Bray + landing.");
	}
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Maybraid Spatial Rumble".into(),
				resolution: (1280, 720).into(),
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
		.insert_resource(demo)
		.add_plugins(SpatialRumblePlaygroundPlugin)
		.run();
}
