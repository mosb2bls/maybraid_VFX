//! Headless preview of continuous health bar states.
//!
//! Captures: full HP, damaged with trail, healing with pulse/preview,
//! healing under 70% cap with marker, and completion flash.
//!
//! ```bash
//! HEALTH_BAR_PREVIEW_DIR=/tmp/health-bar-preview \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a -s "-screen 0 1280x720x24" \
//!   cargo run -p combat-hud --example continuous_health_preview
//! ```

use std::path::PathBuf;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use combat_hud::CombatHudPlugin;
use damage::Health;
use lubikans::{
	LubikanAccountId, LubikanActivationFinished, LubikanActivationOutcome,
	LubikanActivationSessionId, LubikanDefinitionId, LubikanSelfHealChannel,
};
use player::Player;

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Continuous health bar preview".into(),
						resolution: (800, 600).into(),
						..default()
					}),
					..default()
				})
				.disable::<AudioPlugin>(),
		)
		.add_plugins(CombatHudPlugin { health_bars: true, ..default() })
		.add_message::<LubikanActivationFinished>()
		.add_systems(Startup, spawn_preview)
		.add_systems(Update, drive_states)
		.add_systems(Update, capture_shots)
		.add_systems(Update, quit_after_shots)
		.run();
}

#[derive(Component)]
struct PreviewPlayer;

#[derive(Resource)]
struct PreviewDriver {
	frame: u32,
	shots_taken: Vec<String>,
	target_dir: PathBuf,
}

impl Default for PreviewDriver {
	fn default() -> Self {
		let dir = std::env::var("HEALTH_BAR_PREVIEW_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|_| PathBuf::from("/tmp/health-bar-preview"));
		std::fs::create_dir_all(&dir).ok();
		Self { frame: 0, shots_taken: Vec::new(), target_dir: dir }
	}
}

fn spawn_preview(mut commands: Commands) {
	// Camera
	commands.spawn(Camera3d::default());

	// Player entity with health - positioned so bar is visible
	commands
		.spawn((
			PreviewPlayer,
			Player,
			Health { current: 100.0, max: 100.0 },
			Transform::from_xyz(0.0, 1.5, -5.0),
			GlobalTransform::default(),
			player::LocomotionCapsule::default(),
		))
		.insert(Name::new("Preview Player"));

	commands.init_resource::<PreviewDriver>();
	commands.insert_resource(combat_hud::CombatHudVisible(true));
}

fn drive_states(
	mut driver: ResMut<PreviewDriver>,
	mut player: Query<(&mut Health, Option<&mut LubikanSelfHealChannel>), With<PreviewPlayer>>,
	mut commands: Commands,
	player_entity: Query<Entity, With<PreviewPlayer>>,
	mut messages: MessageWriter<lubikans::LubikanActivationFinished>,
) {
	driver.frame += 1;
	let Ok((mut health, _heal_channel)) = player.single_mut() else {
		return;
	};
	let Ok(entity) = player_entity.single() else {
		return;
	};

	// State 1 (frames 1-30): Full health
	if driver.frame == 30 && !driver.shots_taken.contains(&"1-full-health".to_string()) {
		health.current = 100.0;
		commands.entity(entity).remove::<LubikanSelfHealChannel>();
	}

	// State 2 (frames 60-90): Damaged with trail visible
	// Set damage at frame 60, trail catches up over next frames
	if driver.frame == 60 && !driver.shots_taken.contains(&"2-damaged-trail".to_string()) {
		health.current = 50.0;
		commands.entity(entity).remove::<LubikanSelfHealChannel>();
	}

	// State 3 (frames 120-150): Healing mid-way with pulse and preview
	if driver.frame == 120 && !driver.shots_taken.contains(&"3-healing-active".to_string()) {
		health.current = 50.0;
		commands.entity(entity).insert(LubikanSelfHealChannel {
			active: true,
			remaining: 30.0,
			rate: 5.0,
			account: LubikanAccountId::SELF,
			session: LubikanActivationSessionId::mint(),
			damaged: false,
			cap_fraction: 1.0,
		});
	}

	// State 4 (frames 180-210): Healing under 70% cap with marker
	if driver.frame == 180 && !driver.shots_taken.contains(&"4-healing-capped".to_string()) {
		health.current = 40.0;
		commands.entity(entity).insert(LubikanSelfHealChannel {
			active: true,
			remaining: 40.0,
			rate: 5.0,
			account: LubikanAccountId::SELF,
			session: LubikanActivationSessionId::mint(),
			damaged: true,
			cap_fraction: 0.7,
		});
	}

	// State 5 (frames 240-270): Completion flash
	// Trigger flash event at frame 240, capture while flash is visible
	if driver.frame == 240 && !driver.shots_taken.contains(&"5-completion-flash".to_string()) {
		health.current = 70.0;
		commands.entity(entity).remove::<LubikanSelfHealChannel>();
		messages.write(LubikanActivationFinished {
			owner: entity,
			account: LubikanAccountId::SELF,
			definition: LubikanDefinitionId::SELF,
			id: LubikanActivationSessionId::mint(),
			outcome: LubikanActivationOutcome::Completed,
		});
	}
}

fn capture_shots(
	driver: Res<PreviewDriver>,
	mut commands: Commands,
	capturing: Query<&Capturing>,
) {
	if !capturing.is_empty() {
		return;
	}

	let (frame, name) = match driver.frame {
		30 => (30, "1-full-health"),
		90 => (90, "2-damaged-trail"),
		150 => (150, "3-healing-active"),
		210 => (210, "4-healing-capped"),
		245 => (245, "5-completion-flash"),
		_ => return,
	};

	if driver.shots_taken.contains(&name.to_string()) {
		return;
	}

	let path = driver.target_dir.join(format!("{}.png", name));
	info!("Capturing frame {}: {} -> {}", frame, name, path.display());

	commands
		.spawn(Screenshot::primary_window())
		.observe(save_to_disk(path.clone()))
		.observe(move |_trigger: Trigger<bevy::render::view::screenshot::ScreenshotSaved>, mut driver: ResMut<PreviewDriver>| {
			driver.shots_taken.push(name.to_string());
			info!("Saved {}", name);
		});
}

fn quit_after_shots(driver: Res<PreviewDriver>, mut exit: EventWriter<AppExit>) {
	if driver.shots_taken.len() >= 5 {
		info!("All shots captured, exiting");
		exit.send(AppExit::Success);
	}
	if driver.frame > 300 {
		warn!("Timeout reached, exiting with {} shots", driver.shots_taken.len());
		exit.send(AppExit::from_code(1));
	}
}
