//! Headless-friendly Self heal preview: `LubikansPlugin` + `Health` + a UI bar.
//!
//! X11 `scrot` of the Vulkan swapchain stays black on xvfb, so this example
//! GPU-readbacks the primary window via Bevy `Screenshot`.
//!
//! ```bash
//! SELF_HEAL_PREVIEW_DIR=/tmp/self-heal-preview \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   xvfb-run -a cargo run -p lubikans --example self_heal_preview
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use damage::Health;
use lubikans::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationRequest, LubikanLedger,
	LubikanOwner, LubikanSelfHealConfig, LubikansPlugin,
};

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Self heal preview".into(),
						resolution: (640, 360).into(),
						..default()
					}),
					..default()
				})
				.disable::<AudioPlugin>(),
		)
		.add_plugins(LubikansPlugin)
		.insert_resource(LubikanSelfHealConfig {
			rate_frac_per_sec: 0.20,
			duration_secs: 20.0,
			cap_healthy: 1.0,
			cap_damaged: 0.7,
		})
		.add_systems(Startup, spawn_preview)
		.add_systems(
			Update,
			(fund_and_request, sync_health_bar, capture_preview, quit_after_shots).chain(),
		)
		.run();
}

#[derive(Component)]
struct PreviewBarFill;

#[derive(Component)]
struct PreviewStatus;

#[derive(Resource)]
struct PreviewOwner(Entity);

#[derive(Resource, Default)]
struct PreviewPulse(Option<&'static str>);

#[derive(Resource)]
struct PreviewDir(PathBuf);

#[derive(Resource, Default)]
struct PreviewShots {
	healing: bool,
	completed: bool,
}

fn shot_dir() -> PathBuf {
	std::env::var_os("SELF_HEAL_PREVIEW_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/self-heal-preview"))
}

fn spawn_preview(mut commands: Commands) {
	let dir = shot_dir();
	let _ = std::fs::create_dir_all(&dir);
	commands.insert_resource(PreviewDir(dir));
	commands.init_resource::<PreviewPulse>();
	commands.init_resource::<PreviewShots>();
	commands.spawn(Camera2d);
	let owner = commands
		.spawn((LubikanOwner, Health { current: 40.0, max: 100.0 }, Name::new("self-heal-preview")))
		.id();
	commands.insert_resource(PreviewOwner(owner));
	commands
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				padding: UiRect::all(Val::Px(24.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(12.0),
				justify_content: JustifyContent::Center,
				..default()
			},
			BackgroundColor(Color::srgb(0.08, 0.09, 0.11)),
		))
		.with_children(|root| {
			root.spawn((
				PreviewStatus,
				Text::new("Self heal"),
				TextFont { font_size: FontSize::Px(22.0), ..default() },
				TextColor(Color::srgb(0.92, 0.86, 0.55)),
			));
			root.spawn((
				Node {
					width: Val::Px(400.0),
					height: Val::Px(28.0),
					padding: UiRect::all(Val::Px(3.0)),
					border: UiRect::all(Val::Px(1.0)),
					..default()
				},
				BackgroundColor(Color::srgb(0.10, 0.11, 0.13)),
				BorderColor::all(Color::srgb(0.20, 0.21, 0.24)),
			))
			.with_children(|well| {
				well.spawn((
					PreviewBarFill,
					Node { width: Val::Percent(40.0), height: Val::Percent(100.0), ..default() },
					BackgroundColor(Color::srgb(0.35, 0.72, 0.42)),
				));
			});
		});
}

fn fund_and_request(
	mut ledgers: Query<&mut LubikanLedger>,
	owner: Option<Res<PreviewOwner>>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	mut primed: Local<bool>,
) {
	if *primed {
		return;
	}
	let Some(owner) = owner else {
		return;
	};
	let Ok(mut ledger) = ledgers.get_mut(owner.0) else {
		return;
	};
	ledger.apply_to_equipped(25.0);
	requests.write(LubikanActivationRequest::equipped(owner.0));
	*primed = true;
}

fn sync_health_bar(
	owner: Option<Res<PreviewOwner>>,
	healths: Query<&Health>,
	mut finished: MessageReader<LubikanActivationFinished>,
	mut pulse: ResMut<PreviewPulse>,
	mut fills: Query<&mut Node, With<PreviewBarFill>>,
	mut status: Query<&mut Text, With<PreviewStatus>>,
) {
	let Some(owner) = owner else {
		return;
	};
	let Ok(health) = healths.get(owner.0) else {
		return;
	};
	if let Some(event) = finished.read().last() {
		pulse.0 = Some(match event.outcome {
			LubikanActivationOutcome::Completed => "completed -> idle",
			LubikanActivationOutcome::Cancelled => "cancelled -> idle",
		});
	}
	let fraction = health.fraction();
	for mut node in &mut fills {
		node.width = Val::Percent(fraction * 100.0);
	}
	if let Ok(mut text) = status.single_mut() {
		*text = Text::new(format!(
			"Self heal  {current:.0}/{max:.0}  {pulse}",
			current = health.current,
			max = health.max,
			pulse = pulse.0.unwrap_or("healing")
		));
	}
}

fn capture_preview(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<PreviewDir>,
	pulse: Res<PreviewPulse>,
	owner: Option<Res<PreviewOwner>>,
	healths: Query<&Health>,
	mut shots: ResMut<PreviewShots>,
) {
	let Some(owner) = owner else {
		return;
	};
	let Ok(health) = healths.get(owner.0) else {
		return;
	};
	// First presents are often a black swapchain; wait for the bar to move.
	if !shots.healing && time.elapsed_secs() >= 1.25 && (55.0..95.0).contains(&health.current) {
		request_shot(&mut commands, dir.0.join("01-healing.png"));
		shots.healing = true;
		info!(
			"preview shot healing at {current:.0}/{max:.0}",
			current = health.current,
			max = health.max
		);
	}
	if !shots.completed && pulse.0 == Some("completed -> idle") && time.elapsed_secs() >= 1.5 {
		request_shot(&mut commands, dir.0.join("02-completed-idle.png"));
		shots.completed = true;
		info!(
			"preview shot completed at {current:.0}/{max:.0}",
			current = health.current,
			max = health.max
		);
	}
}

fn request_shot(commands: &mut Commands, path: impl AsRef<Path>) {
	let path = path.as_ref().to_owned();
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn quit_after_shots(
	time: Res<Time>,
	shots: Res<PreviewShots>,
	dir: Res<PreviewDir>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	let files_ready = shots.healing
		&& shots.completed
		&& dir.0.join("01-healing.png").is_file()
		&& dir.0.join("02-completed-idle.png").is_file();
	if files_ready && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(250))
		|| time.elapsed() > Duration::from_secs(16)
	{
		exit.write(AppExit::Success);
	}
}
