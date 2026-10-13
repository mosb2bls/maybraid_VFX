//! Minimal Standard / Tanzer mode indicator.

use bevy::prelude::*;

use crate::{ControlMode, TanzUser};

#[derive(Component)]
pub struct TanzModeHud;

#[derive(Component)]
pub struct TanzModeHudLabel;

pub fn spawn_tanz_mode_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("tanz-mode-hud"),
			TanzModeHud,
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(18.0),
				right: Val::Px(18.0),
				padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.55)),
			GlobalZIndex(24),
			Pickable::IGNORE,
			Visibility::Hidden,
		))
		.with_children(|parent| {
			parent.spawn((
				TanzModeHudLabel,
				Text::new("STANDARD"),
				TextFont { font_size: FontSize::Px(14.0), ..default() },
				TextColor(Color::srgb(0.92, 0.9, 0.82)),
				Pickable::IGNORE,
			));
		});
}

pub fn sync_tanz_mode_hud(
	users: Query<&TanzUser>,
	mut roots: Query<&mut Visibility, With<TanzModeHud>>,
	mut labels: Query<&mut Text, With<TanzModeHudLabel>>,
) {
	let mode = users.iter().next().map(|user| user.mode);
	for mut visibility in &mut roots {
		*visibility = if mode.is_some() { Visibility::Visible } else { Visibility::Hidden };
	}
	let Some(mode) = mode else {
		return;
	};
	let label = match mode {
		ControlMode::Standard => "STANDARD",
		ControlMode::Tanzer => "TANZER",
	};
	for mut text in &mut labels {
		if text.0 != label {
			text.0 = String::from(label);
		}
	}
}
