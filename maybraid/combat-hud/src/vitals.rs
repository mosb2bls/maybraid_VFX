//! Bottom-left player name + segmented health bar.

use bevy::prelude::*;
use damage::Health;
use menu_components::{
	spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_SALMON, TEXT_YELLOW,
};
use player::Player;

use crate::plate::{spawn_hud_plate, HudPlateCorner, HudPlateLayout};

pub(crate) const VITALS_WIDTH: f32 = 228.0;
const WELL_FILL: Color = Color::srgb(0.10, 0.11, 0.13);
const WELL_STROKE: Color = Color::srgba(0.08, 0.08, 0.10, 0.95);
const PIP_EMPTY: Color = Color::srgb(0.18, 0.19, 0.22);
const PIP_HEIGHT: f32 = 14.0;
const PIP_RADIUS: f32 = 3.0;

pub(crate) const VITALS_PIPS: usize = 10;
pub(crate) const DEFAULT_VITALS_NAME: &str = "Player";

#[derive(Component)]
pub(crate) struct PlayerVitalsRoot;

#[derive(Component)]
pub(crate) struct PlayerVitalsName;

#[derive(Component)]
pub(crate) struct PlayerVitalsPip(pub u8);

pub(crate) fn vitals_fonts(asset_server: Option<&AssetServer>) -> HudFonts {
	match asset_server {
		Some(server) => HudFonts::load(server),
		None => HudFonts {
			black: Handle::default(),
			semibold: Handle::default(),
			regular: Handle::default(),
			logo: Handle::default(),
		},
	}
}

pub(crate) fn spawn_player_vitals(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	spawn_hud_plate(
		parent,
		"player-vitals",
		PlayerVitalsRoot,
		HudPlateLayout { width: VITALS_WIDTH, row_gap: 6.0, corner: HudPlateCorner::BottomLeft },
		|plate| {
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					PlayerVitalsName,
					Text::new(DEFAULT_VITALS_NAME),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			plate
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(PIP_HEIGHT + 6.0),
						padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
						column_gap: Val::Px(3.0),
						align_items: AlignItems::Center,
						border: UiRect::all(Val::Px(1.0)),
						border_radius: BorderRadius::all(Val::Px(PIP_RADIUS + 2.0)),
						..default()
					},
					BackgroundColor(WELL_FILL),
					BorderColor::all(WELL_STROKE),
					Pickable::IGNORE,
				))
				.with_children(|well| {
					for index in 0..VITALS_PIPS {
						well.spawn((
							PlayerVitalsPip(index as u8),
							Node {
								height: Val::Px(PIP_HEIGHT),
								flex_grow: 1.0,
								border: UiRect::all(Val::Px(1.0)),
								border_radius: BorderRadius::all(Val::Px(PIP_RADIUS)),
								..default()
							},
							BackgroundColor(PIP_EMPTY),
							BorderColor::all(WELL_STROKE),
							Pickable::IGNORE,
						));
					}
				});
		},
	);
}

pub(crate) fn sync_player_vitals(
	hud: Res<crate::CombatHudVisible>,
	players: Query<(&Health, Option<&Name>), With<Player>>,
	mut roots: Query<&mut Visibility, With<PlayerVitalsRoot>>,
	mut names: Query<&mut Text, With<PlayerVitalsName>>,
	mut pips: Query<(&PlayerVitalsPip, &mut BackgroundColor)>,
) {
	let Some((health, name)) = players.iter().next().filter(|_| hud.0) else {
		hide_vitals(&mut roots);
		return;
	};
	let label = display_name(name.map(|name| name.as_str()));
	let fraction = health.fraction();
	let fill = vitals_fill_color(fraction);
	for mut visibility in &mut roots {
		*visibility = Visibility::Visible;
	}
	for mut text in &mut names {
		if text.0 != label {
			text.0 = label.clone();
		}
	}
	for (pip, mut color) in &mut pips {
		color.0 = if pip_lit(fraction, pip.0 as usize) { fill } else { PIP_EMPTY };
	}
}

fn hide_vitals(roots: &mut Query<&mut Visibility, With<PlayerVitalsRoot>>) {
	for mut visibility in roots {
		*visibility = Visibility::Hidden;
	}
}

pub(crate) fn display_name(name: Option<&str>) -> String {
	name.map(str::trim)
		.filter(|name| !name.is_empty())
		.unwrap_or(DEFAULT_VITALS_NAME)
		.to_string()
}

pub(crate) fn pip_lit(fraction: f32, index: usize) -> bool {
	fraction > index as f32 / VITALS_PIPS as f32
}

pub(crate) fn vitals_fill_color(fraction: f32) -> Color {
	if fraction <= 0.0 {
		PIP_EMPTY
	} else if fraction < 0.35 {
		TEXT_SALMON
	} else if fraction < 0.7 {
		Color::srgb(0.95, 0.62, 0.22)
	} else {
		TEXT_YELLOW
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_name_falls_back_to_player() {
		assert_eq!(display_name(None), "Player");
		assert_eq!(display_name(Some("   ")), "Player");
		assert_eq!(display_name(Some("Ada")), "Ada");
	}

	#[test]
	fn pips_light_from_the_left() {
		assert!(pip_lit(0.4, 0));
		assert!(pip_lit(0.4, 3));
		assert!(!pip_lit(0.4, 4));
		assert!(!pip_lit(0.0, 0));
	}

	#[test]
	fn full_bar_uses_maybraid_yellow() {
		assert_eq!(vitals_fill_color(1.0), TEXT_YELLOW);
		assert_eq!(vitals_fill_color(0.2), TEXT_SALMON);
	}
}
