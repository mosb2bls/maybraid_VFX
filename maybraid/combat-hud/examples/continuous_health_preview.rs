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
use bevy::winit::WinitSettings;
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
						resolution: (640, 360).into(),
						..default()
					}),
					..default()
				})
				.disable::<AudioPlugin>(),
		)
		.insert_resource(WinitSettings::continuous())
		.add_plugins(CombatHudPlugin {
			health_bars: true,
			hit_markers: false,
			directional_damage: false,
			player_vitals: false,
			score: false,
		})
		.add_message::<LubikanActivationFinished>()
		.add_message::<AppExit>()
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
	target_dir: PathBuf,
}

impl Default for PreviewDriver {
	fn default() -> Self {
		let dir = std::env::var("HEALTH_BAR_PREVIEW_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|_| PathBuf::from("/tmp/health-bar-preview"));
		std::fs::create_dir_all(&dir).ok();
		Self { frame: 0, target_dir: dir }
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

	// State 1 (frames 10-40): Full health
	if driver.frame == 10 {
		health.current = 100.0;
		commands.entity(entity).remove::<LubikanSelfHealChannel>();
	}

	// State 2 (frames 50-80): Damaged with trail visible
	if driver.frame == 50 {
		health.current = 50.0;
		commands.entity(entity).remove::<LubikanSelfHealChannel>();
	}

	// State 3 (frames 90-120): Healing mid-way with pulse and preview
	if driver.frame == 90 {
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

	// State 4 (frames 130-160): Healing under 70% cap with marker
	if driver.frame == 130 {
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

	// State 5 (frames 170-200): Completion flash
	if driver.frame == 170 {
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
		40 => (40, "1-full-health"),
		80 => (80, "2-damaged-trail"),
		120 => (120, "3-healing-active"),
		160 => (160, "4-healing-capped"),
		180 => (180, "5-completion-flash"),
		_ => return,
	};

	// Don't re-capture if file already exists
	if driver.target_dir.join(format!("{}.png", name)).is_file() {
		return;
	}

	let path = driver.target_dir.join(format!("{}.png", name));
	info!("Capturing frame {}: {} -> {}", frame, name, path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn quit_after_shots(
	driver: Res<PreviewDriver>,
	mut exit: MessageWriter<AppExit>,
	capturing: Query<(), With<Capturing>>,
) {
	// Check if all expected files exist
	let shot1 = driver.target_dir.join("1-full-health.png").is_file();
	let shot2 = driver.target_dir.join("2-damaged-trail.png").is_file();
	let shot3 = driver.target_dir.join("3-healing-active.png").is_file();
	let shot4 = driver.target_dir.join("4-healing-capped.png").is_file();
	let shot5 = driver.target_dir.join("5-completion-flash.png").is_file();

	let all_done = shot1 && shot2 && shot3 && shot4 && shot5 && capturing.is_empty();

	if all_done {
		info!("All 5 shots captured, exiting successfully");
		exit.write(AppExit::Success);
	}
	if driver.frame > 220 {
		let count = [shot1, shot2, shot3, shot4, shot5].iter().filter(|&&x| x).count();
		warn!("Frame limit reached with {} shots, exiting", count);
		exit.write(AppExit::Success);
	}
}
