//! Firing-range-specific additions to the shared combat HUD.

use bevy::prelude::*;
use evasion_intelligence::EvasionIntelligenceUser;
use game_commands::GameCommandDrawerVisible;
use player::Player;

use crate::session::CombatantKit;

const GUN_CARD_WIDTH: f32 = 248.0;
const DRAWER_CLEARANCE: f32 = 290.0;
const FLEE_DOT: Color = Color::srgb(1.0, 0.38, 0.72);
const HIDE_DOT: Color = Color::srgb(0.28, 0.52, 1.0);

#[derive(Component)]
pub(crate) struct GunStatsPanel;

#[derive(Component)]
pub(crate) struct GunStatsText;

pub(crate) fn spawn_firing_range_hud(mut commands: Commands) {
	commands
		.spawn((
			Name::new("firing-range-hud"),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			GlobalZIndex(i32::MAX - 8),
			Pickable::IGNORE,
		))
		.with_children(spawn_gun_stats_card);
}

fn spawn_gun_stats_card(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			Name::new("gun-stats"),
			GunStatsPanel,
			Node {
				position_type: PositionType::Absolute,
				right: Val::Px(16.0),
				bottom: Val::Px(16.0),
				min_width: Val::Px(GUN_CARD_WIDTH),
				flex_direction: FlexDirection::Column,
				padding: UiRect::all(Val::Px(10.0)),
				border_radius: BorderRadius::all(Val::Px(4.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
			Pickable::IGNORE,
		))
		.with_children(|card| {
			card.spawn((
				GunStatsText,
				Text::new(""),
				TextFont { font_size: FontSize::Px(13.0), ..default() },
				TextColor(Color::srgb(0.92, 0.95, 0.98)),
			));
		});
}

pub(crate) fn sync_gun_stats(
	drawer: Res<GameCommandDrawerVisible>,
	players: Query<Option<&CombatantKit>, With<Player>>,
	mut panels: Query<(&mut Node, &mut Visibility), With<GunStatsPanel>>,
	mut labels: Query<&mut Text, With<GunStatsText>>,
) {
	let Ok((mut node, mut visibility)) = panels.single_mut() else {
		return;
	};
	let Ok(mut text) = labels.single_mut() else {
		return;
	};
	node.bottom = if drawer.0 { Val::Px(DRAWER_CLEARANCE) } else { Val::Px(16.0) };
	let Ok(kit) = players.single() else {
		*visibility = Visibility::Hidden;
		return;
	};
	*visibility = Visibility::Visible;
	text.0 = gun_card_text(kit);
}

fn gun_card_text(kit: Option<&CombatantKit>) -> String {
	match kit {
		Some(kit) => format_gun_card(
			kit.firearm.body.label(),
			&kit.stats.catalog_detail(),
			&live_stat_rows(kit),
		),
		None => format_gun_card("bullpup", "Bolt · 25 DPC", &duel_stat_rows()),
	}
}

fn live_stat_rows(kit: &CombatantKit) -> Vec<(String, String)> {
	let live = kit.live.payload.amount;
	let catalog = f32::from(kit.stats.damage);
	kit.stats
		.stat_rows()
		.into_iter()
		.map(|(label, value)| {
			if label == "DPC" && (live - catalog).abs() >= 0.5 {
				(label, format!("{live:.0} ({catalog:.0})"))
			} else {
				(label, value)
			}
		})
		.collect()
}

fn duel_stat_rows() -> Vec<(String, String)> {
	vec![
		(String::from("Projectile"), String::from("Bolt")),
		(String::from("Speed"), String::from("180/s")),
		(String::from("Penetration"), String::from("0.85")),
		(String::from("Range"), String::from("36 m")),
		(String::from("Fire"), String::from("Auto")),
		(String::from("DPC"), String::from("25")),
	]
}

fn format_gun_card(title: &str, detail: &str, rows: &[(String, String)]) -> String {
	let mut lines = vec![title.to_ascii_uppercase(), String::from(detail), String::new()];
	for (label, value) in rows {
		lines.push(format!("{label:<13}{value}"));
	}
	lines.join("\n")
}

fn evasion_dot_color(evasion: &EvasionIntelligenceUser) -> Option<Color> {
	if evasion.signal.is_flee() {
		Some(FLEE_DOT)
	} else if evasion.signal.is_hide() {
		Some(HIDE_DOT)
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn duel_card_lists_the_default_bolt() {
		let text = gun_card_text(None);
		assert!(text.starts_with("BULLPUP"), "{text}");
		assert!(text.contains("Bolt · 25 DPC"), "{text}");
		assert!(text.contains("180/s"), "{text}");
		assert!(text.contains("36 m"), "{text}");
	}

	#[test]
	fn flee_dot_is_pink_and_hide_dot_is_blue() {
		let mut fleeing = EvasionIntelligenceUser::default();
		fleeing.signal.actuator = evasion_intelligence::EvasionActuator::Flee;
		let mut hiding = EvasionIntelligenceUser::default();
		hiding.signal.actuator = evasion_intelligence::EvasionActuator::Hide;
		let flee = evasion_dot_color(&fleeing).expect("flee color").to_srgba();
		let hide = evasion_dot_color(&hiding).expect("hide color").to_srgba();
		assert!(flee.red > flee.blue && flee.red > flee.green);
		assert!(hide.blue > hide.red && hide.blue > hide.green);
		assert!(evasion_dot_color(&EvasionIntelligenceUser::default()).is_none());
	}
}
