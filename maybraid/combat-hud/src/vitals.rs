//! Bottom-left player name + continuous health bar.

use bevy::prelude::*;
use damage::Downed;
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::continuous_health::{
	spawn_continuous_health_bar_in_builder, ContinuousHealthBar, ContinuousHealthConfig,
	HealthAnimState,
};
use crate::plate::{spawn_hud_plate, HudPlateCorner, HudPlateLayout};

pub(crate) const VITALS_WIDTH: f32 = 228.0;
pub(crate) const DEFAULT_VITALS_NAME: &str = "Player";

#[derive(Component)]
pub(crate) struct PlayerVitalsRoot;

#[derive(Component)]
pub(crate) struct PlayerVitalsName;

#[derive(Component)]
pub struct PlayerVitalsHealthBar;

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
		HudPlateLayout {
			width: VITALS_WIDTH,
			row_gap: 6.0,
			corner: HudPlateCorner::BottomLeft,
			bottom_lift: 0.0,
		},
		|plate| {
			let bar_entity = spawn_continuous_health_bar_in_builder(plate, player, config);
			plate.commands().entity(bar_entity).insert(PlayerVitalsHealthBar);
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					PlayerVitalsName,
					Text::new(DEFAULT_VITALS_NAME),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
		},
	);
}

pub(crate) fn sync_player_vitals(
	hud: Res<crate::CombatHudVisible>,
	players: Query<(Entity, Option<&Name>), (With<Player>, Without<Downed>)>,
	mut roots: Query<&mut Visibility, With<PlayerVitalsRoot>>,
	mut names: Query<&mut Text, With<PlayerVitalsName>>,
	mut health_bars: Query<
		(&mut ContinuousHealthBar, &mut HealthAnimState),
		With<PlayerVitalsHealthBar>,
	>,
	healths: Query<&damage::Health>,
) {
	let Some((player_entity, name)) = players.iter().next().filter(|_| hud.0) else {
		hide_vitals(&mut roots);
		return;
	};
	let label = display_name(name.map(|name| name.as_str()));
	for mut visibility in &mut roots {
		if *visibility != Visibility::Visible {
			*visibility = Visibility::Visible;
		}
	}
	for mut text in &mut names {
		if text.0 != label {
			text.0 = label.clone();
		}
	}
	for (mut health_bar, mut anim) in &mut health_bars {
		if health_bar.target != player_entity {
			health_bar.target = player_entity;
			*anim = HealthAnimState::snap_to(
				healths.get(player_entity).map(|health| health.fraction()).unwrap_or(1.0),
			);
		}
	}
}

fn hide_vitals(roots: &mut Query<&mut Visibility, With<PlayerVitalsRoot>>) {
	for mut visibility in roots {
		if *visibility != Visibility::Hidden {
			*visibility = Visibility::Hidden;
		}
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
	fn downed_player_hides_the_vitals_plate() {
		use crate::CombatHudPlugin;
		use crate::CombatHudVisible;

		let mut app = App::new();
		app.add_plugins(bevy::MinimalPlugins);
		app.add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});
		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		let player =
			app.world_mut().spawn((Player, Health::from_max(100.0), Name::new("Ada"))).id();
		app.update();
		{
			let visibility = app
				.world_mut()
				.query_filtered::<&Visibility, With<PlayerVitalsRoot>>()
				.iter(app.world())
				.next()
				.copied()
				.expect("plate");
			assert_eq!(visibility, Visibility::Visible);
		}
		app.world_mut().entity_mut(player).insert(Downed {
			source: None,
			point: Vec3::ZERO,
			at: 0.0,
		});
		app.update();
		let visibility = app
			.world_mut()
			.query_filtered::<&Visibility, With<PlayerVitalsRoot>>()
			.iter(app.world())
			.next()
			.copied()
			.expect("plate");
		assert_eq!(visibility, Visibility::Hidden);
	}

	#[test]
	fn vitals_continuous_bar_targets_player() {
		// Test that spawned health bar has correct layout and target
		use crate::CombatHudPlugin;
		use bevy::app::App;

		let mut app = App::new();
		app.add_plugins(bevy::MinimalPlugins);
		app.add_plugins(CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		});

		// Spawn player
		let player = app
			.world_mut()
			.spawn((Player, Health::from_max(100.0), Name::new("TestPlayer")))
			.id();

		// Run one update to spawn HUD
		app.update();

		// Find the UI child health bar
		let mut query = app
			.world_mut()
			.query_filtered::<&ContinuousHealthBar, With<ContinuousHealthBar>>();
		let bars: Vec<_> = query.iter(app.world()).collect();

		assert_eq!(bars.len(), 1, "should have one player health bar");
		assert_eq!(bars[0].target, player, "bar should target the player entity");
	}

	fn fill_percent(app: &mut App) -> f32 {
		use crate::HealthFill;

		let mut query = app.world_mut().query_filtered::<&Node, With<HealthFill>>();
		match query.iter(app.world()).next().expect("HealthFill").width {
			Val::Percent(percent) => percent,
			other => panic!("fill should use percent width, got {other:?}"),
		}
	}

	#[test]
	fn vitals_fill_drops_after_health_falls() {
		use crate::CombatHudPlugin;
		use crate::CombatHudVisible;
		use bevy::time::{TimeUpdateStrategy, Virtual};
		use std::time::Duration;

		const DT: f32 = 0.2;

		let mut app = App::new();
		app.add_plugins(bevy::MinimalPlugins)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
			.add_plugins(CombatHudPlugin {
				hit_markers: false,
				directional_damage: false,
				player_vitals: true,
				lubikan_charge: false,
				lubikan_point_feedback: false,
				score: false,
			});
		app.world_mut()
			.resource_mut::<Time<Virtual>>()
			.set_max_delta(Duration::from_secs(1));
		app.world_mut().insert_resource(CombatHudVisible(true));
		let player = app
			.world_mut()
			.spawn((Player, Health::from_max(100.0), Name::new("Ada")))
			.id();
		app.update();

		let baseline = fill_percent(&mut app);
		assert!(
			(baseline - 100.0).abs() < 1.0,
			"full health should fill the bar, got {baseline}"
		);

		app.world_mut().get_mut::<Health>(player).expect("health").current = 40.0;
		app.update();

		let after = fill_percent(&mut app);
		assert!(
			after < baseline - 5.0,
			"fill should interpolate toward the new fraction, baseline={baseline} after={after}"
		);
		assert!(after > 0.0, "partial health should still show a fill");
	}
}
