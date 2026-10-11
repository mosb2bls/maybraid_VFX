//! Bottom-left player name + continuous health bar.

use bevy::prelude::*;
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::continuous_health::{
	CapMarker, CompletionFlash, ContinuousHealthBar, ContinuousHealthConfig, HealPreview,
	HealthAnimState, HealthFill, HealthLabel, HealthStatusDot, HealthTrail,
};
use crate::plate::{spawn_hud_plate, HudPlateCorner, HudPlateLayout};

pub(crate) const VITALS_WIDTH: f32 = 228.0;
pub(crate) const DEFAULT_VITALS_NAME: &str = "Player";

#[derive(Component)]
pub(crate) struct PlayerVitalsRoot;

#[derive(Component)]
pub(crate) struct PlayerVitalsName;

#[derive(Component)]
pub(crate) struct PlayerVitalsHealthBar;

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

pub(crate) fn spawn_player_vitals(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	config: &ContinuousHealthConfig,
	player: Entity,
) {
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
			// Spawn continuous health bar (inline version with marker component)
			spawn_player_vitals_health_bar(plate, player, config);
		},
	);
}

fn spawn_player_vitals_health_bar(
	parent: &mut ChildSpawnerCommands,
	target: Entity,
	config: &ContinuousHealthConfig,
) {
	parent
		.spawn((
			ContinuousHealthBar { target },
			HealthAnimState::default(),
			PlayerVitalsHealthBar,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Px(config.bar_width),
				height: Val::Px(config.bar_height + 20.0),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				row_gap: Val::Px(2.0),
				..default()
			},
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|anchor| {
			// Status dot
			anchor.spawn((
				HealthStatusDot,
				Node {
					width: Val::Px(10.0),
					height: Val::Px(10.0),
					border_radius: BorderRadius::all(Val::Px(10.0)),
					..default()
				},
				BackgroundColor(Color::WHITE),
				Visibility::Hidden,
				Pickable::IGNORE,
			));
			
			// Numeric label
			anchor.spawn((
				HealthLabel,
				Text::new(""),
				TextFont { font_size: FontSize::Px(12.0), ..default() },
				TextColor(Color::WHITE),
			));
			
			// Bar container with frame
			anchor
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(config.bar_height),
						padding: UiRect::all(Val::Px(config.frame_thickness)),
						border_radius: BorderRadius::all(Val::Px(config.corner_radius)),
						..default()
					},
					BackgroundColor(Color::srgba(0.12, 0.13, 0.15, 0.9)),
					Pickable::IGNORE,
				))
				.with_children(|frame| {
					frame
						.spawn((
							Node {
								width: Val::Percent(100.0),
								height: Val::Percent(100.0),
								border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
								overflow: Overflow::clip(),
								..default()
							},
							BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.95)),
							Pickable::IGNORE,
						))
						.with_children(|track| {
							// Damage trail (behind fill)
							track.spawn((
								HealthTrail,
								Node {
									width: Val::Percent(100.0),
									height: Val::Percent(100.0),
									position_type: PositionType::Absolute,
									left: Val::Px(0.0),
									top: Val::Px(0.0),
									border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
									..default()
								},
								BackgroundColor(config.trail_color),
								Pickable::IGNORE,
							));
							
							// Main fill
							track.spawn((
								HealthFill,
								Node {
									width: Val::Percent(100.0),
									height: Val::Percent(100.0),
									position_type: PositionType::Absolute,
									left: Val::Px(0.0),
									top: Val::Px(0.0),
									border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
									..default()
								},
								BackgroundColor(config.fill_color_high),
								Pickable::IGNORE,
							));
							
							// Heal preview
							track.spawn((
								HealPreview,
								Node {
									width: Val::Percent(0.0),
									height: Val::Percent(100.0),
									position_type: PositionType::Absolute,
									left: Val::Px(0.0),
									top: Val::Px(0.0),
									border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
									..default()
								},
								BackgroundColor(config.heal_preview_color),
								Visibility::Hidden,
								Pickable::IGNORE,
			));
							
							// Cap marker
							track.spawn((
								CapMarker,
								Node {
									width: Val::Px(config.cap_marker_width),
									height: Val::Percent(100.0),
									position_type: PositionType::Absolute,
									left: Val::Px(0.0),
									top: Val::Px(0.0),
									..default()
								},
								BackgroundColor(config.cap_marker_color),
								Visibility::Hidden,
								Pickable::IGNORE,
							));
							
							// Completion flash
							track.spawn((
								CompletionFlash,
								Node {
									width: Val::Percent(100.0),
									height: Val::Percent(100.0),
									position_type: PositionType::Absolute,
									left: Val::Px(0.0),
									top: Val::Px(0.0),
									border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
									..default()
								},
								BackgroundColor(config.completion_flash_color.with_alpha(0.0)),
								Pickable::IGNORE,
							));
						});
				});
		});
}

pub(crate) fn sync_player_vitals(
	hud: Res<crate::CombatHudVisible>,
	players: Query<(Entity, Option<&Name>), With<Player>>,
	mut roots: Query<&mut Visibility, With<PlayerVitalsRoot>>,
	mut names: Query<&mut Text, With<PlayerVitalsName>>,
	mut health_bars: Query<&mut crate::ContinuousHealthBar, With<PlayerVitalsHealthBar>>,
) {
	let Some((player_entity, name)) = players.iter().next().filter(|_| hud.0) else {
		hide_vitals(&mut roots);
		return;
	};
	let label = display_name(name.map(|name| name.as_str()));
	for mut visibility in &mut roots {
		*visibility = Visibility::Visible;
	}
	for mut text in &mut names {
		if text.0 != label {
			text.0 = label.clone();
		}
	}
	// Update health bar target if needed
	for mut health_bar in &mut health_bars {
		if health_bar.target != player_entity {
			health_bar.target = player_entity;
		}
	}
}

fn hide_vitals(roots: &mut Query<&mut Visibility, With<PlayerVitalsRoot>>) {
	for mut visibility in roots {
		*visibility = Visibility::Hidden;
	}
}

fn display_name(name: Option<&str>) -> String {
	name.map(str::trim)
		.filter(|name| !name.is_empty())
		.unwrap_or(DEFAULT_VITALS_NAME)
		.to_string()
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
}
