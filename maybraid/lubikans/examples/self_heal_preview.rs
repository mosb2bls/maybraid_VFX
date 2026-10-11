//! Headless-friendly Self heal preview: `LubikansPlugin` + `Health` + a UI bar.
//!
//! ```bash
//! cargo run -p lubikans --example self_heal_preview
//! ```

use std::time::Duration;

use bevy::prelude::*;
use damage::Health;
use lubikans::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationRequest, LubikanLedger,
	LubikanOwner, LubikansPlugin,
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Self heal preview".into(),
				resolution: (640, 360).into(),
				..default()
			}),
			..default()
		}))
		.add_plugins(LubikansPlugin)
		.add_systems(Startup, spawn_preview)
		.add_systems(Update, (fund_and_request, sync_health_bar, quit_after_finish).chain())
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

fn spawn_preview(mut commands: Commands) {
	commands.spawn(Camera2d);
	commands.init_resource::<PreviewPulse>();
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
			LubikanActivationOutcome::Completed => "completed → idle",
			LubikanActivationOutcome::Cancelled => "cancelled → idle",
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

fn quit_after_finish(
	time: Res<Time>,
	pulse: Res<PreviewPulse>,
	mut exit: MessageWriter<AppExit>,
	mut closed_at: Local<Option<Duration>>,
) {
	if pulse.0 == Some("completed → idle") && closed_at.is_none() {
		*closed_at = Some(time.elapsed());
	}
	if closed_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(400))
		|| time.elapsed() > Duration::from_secs(16)
	{
		exit.write(AppExit::Success);
	}
}
