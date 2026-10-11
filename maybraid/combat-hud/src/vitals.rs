//! Bottom-left player name + continuous health bar.

use bevy::prelude::*;
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::continuous_health::{
	spawn_continuous_health_bar_in_builder, ContinuousHealthBar, ContinuousHealthConfig,
	HealthBarLayout,
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
			
			// Spawn continuous health bar as UI child with marker
			let bar_entity = spawn_continuous_health_bar_in_builder(
				plate,
				player,
				config,
				HealthBarLayout::UiChild,
			);
			
			// Add PlayerVitalsHealthBar marker using commands
			plate.commands().entity(bar_entity).insert(PlayerVitalsHealthBar);
		},
	);
}

pub(crate) fn sync_player_vitals(
	hud: Res<crate::CombatHudVisible>,
	players: Query<(Entity, Option<&Name>), With<Player>>,
	mut roots: Query<&mut Visibility, With<PlayerVitalsRoot>>,
	mut names: Query<&mut Text, With<PlayerVitalsName>>,
	mut health_bars: Query<&mut ContinuousHealthBar, With<PlayerVitalsHealthBar>>,
) {
	let Some((player_entity, name)) = players.iter().next().filter(|_| hud.0) else {
		hide_vitals(&mut roots);
		return;
	};
	info!(
		"[vitals] sync_player_vitals: player_entity={:?} name={:?} hud_visible={}",
		player_entity, name.map(|n| n.as_str()), hud.0
	);
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
			info!(
				"[vitals] Updating health bar target from {:?} to {:?}",
				health_bar.target, player_entity
			);
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
	use damage::Health;

	#[test]
	fn empty_name_falls_back_to_player() {
		assert_eq!(display_name(None), "Player");
		assert_eq!(display_name(Some("   ")), "Player");
		assert_eq!(display_name(Some("Ada")), "Ada");
	}

	#[test]
	fn vitals_continuous_bar_targets_player() {
		// Test that spawned health bar has correct layout and target
		use crate::CombatHudPlugin;
		use bevy::app::App;
		
		let mut app = App::new();
		app.add_plugins(bevy::MinimalPlugins);
		app.add_plugins(CombatHudPlugin {
			health_bars: false,
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});
		
		// Spawn player
		let player = app.world_mut().spawn((
			Player,
			Health::from_max(100.0),
			Name::new("TestPlayer"),
		)).id();
		
		// Run one update to spawn HUD
		app.update();
		
		// Find the UI child health bar
		let mut query = app.world_mut().query_filtered::<&ContinuousHealthBar, With<ContinuousHealthBar>>();
		let bars: Vec<_> = query
			.iter(app.world())
			.filter(|bar| bar.layout == HealthBarLayout::UiChild)
			.collect();
		
		assert_eq!(bars.len(), 1, "should have one UI child health bar");
		assert_eq!(bars[0].target, player, "bar should target the player entity");
	}
}
