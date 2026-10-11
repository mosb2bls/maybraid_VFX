//! Top-right running score: the hit-marker points, downs, deaths, and streak,
//! plus the live enemy count when the mode keeps one.

use bevy::prelude::*;
use damage::DamageApplied;
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::plate::{spawn_hud_plate, HudPlateCorner, HudPlateLayout};
use crate::CombatHudVisible;

const SCORE_WIDTH: f32 = 168.0;
const SCORE_LINE_PX: f32 = 13.0;
const SCORE_LINE: Color = Color::srgb(0.95, 0.96, 0.98);

/// Training tally. Charge lives on the equipped Lubikan; this resource only
/// keeps downs, deaths, and streak. A mode inserts it; a fresh insert resets
/// it, and removing it hides the panel.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatScore {
	pub downs: u32,
	pub deaths: u32,
	/// Downs since the player last died.
	pub streak: u32,
	pub best_streak: u32,
}

impl CombatScore {
	pub fn record_down(&mut self) {
		self.downs = self.downs.saturating_add(1);
		self.streak = self.streak.saturating_add(1);
		self.best_streak = self.best_streak.max(self.streak);
	}

	pub fn record_death(&mut self) {
		self.deaths = self.deaths.saturating_add(1);
		self.streak = 0;
	}

	fn headline(&self) -> String {
		format!("Downs  {}", self.downs)
	}

	fn tally(&self) -> String {
		format!("Downs  {}    Deaths  {}", self.downs, self.deaths)
	}

	fn streaks(&self) -> String {
		format!("Streak  {}    Best  {}", self.streak, self.best_streak)
	}
}

/// Enemies still standing. The mode that fields them keeps this current; the
/// score panel shows it while both it and a [`CombatScore`] exist.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiveEnemies(pub u32);

impl LiveEnemies {
	fn line(self) -> String {
		format!("Enemies  {}", self.0)
	}
}

#[derive(Component)]
pub(crate) struct CombatScoreRoot;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CombatScoreLine {
	Headline,
	Enemies,
	Tally,
	Streaks,
}

impl CombatScoreLine {
	/// `None` hides the line.
	fn text(self, score: &CombatScore, enemies: Option<LiveEnemies>) -> Option<String> {
		match self {
			Self::Headline => Some(score.headline()),
			Self::Enemies => enemies.map(LiveEnemies::line),
			Self::Tally => Some(score.tally()),
			Self::Streaks => Some(score.streaks()),
		}
	}
}

pub(crate) fn spawn_combat_score(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	spawn_hud_plate(
		parent,
		"combat-score",
		CombatScoreRoot,
		HudPlateLayout { width: SCORE_WIDTH, row_gap: 4.0, corner: HudPlateCorner::TopRight },
		|plate| {
			let score = CombatScore::default();
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					CombatScoreLine::Headline,
					Text::new(score.headline()),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			for line in [CombatScoreLine::Enemies, CombatScoreLine::Tally, CombatScoreLine::Streaks]
			{
				let text = line.text(&score, None);
				plate.spawn((
					line,
					Node {
						display: if text.is_some() { Display::Flex } else { Display::None },
						..default()
					},
					Text::new(text.unwrap_or_default()),
					fonts.body(SCORE_LINE_PX),
					TextColor(SCORE_LINE),
					Pickable::IGNORE,
				));
			}
		},
	);
}

/// Runs between damage apply and down, while a downed player still carries
/// [`Player`].
pub(crate) fn ingest_combat_score(
	score: Option<ResMut<CombatScore>>,
	mut hits: MessageReader<DamageApplied>,
	players: Query<Entity, With<Player>>,
) {
	let (Some(mut score), Ok(player)) = (score, players.single()) else {
		hits.clear();
		return;
	};
	for hit in hits.read() {
		if hit.target == player {
			if hit.remaining <= 0.0 {
				score.record_death();
			}
			continue;
		}
		if hit.source != Some(player) {
			continue;
		}
		if hit.remaining <= 0.0 {
			score.record_down();
		}
	}
}

pub(crate) fn sync_combat_score(
	hud: Res<CombatHudVisible>,
	score: Option<Res<CombatScore>>,
	enemies: Option<Res<LiveEnemies>>,
	mut roots: Query<&mut Visibility, With<CombatScoreRoot>>,
	mut lines: Query<(&CombatScoreLine, &mut Text, &mut Node)>,
) {
	let shown = hud.0 && score.is_some();
	for mut visibility in &mut roots {
		let wanted = if shown { Visibility::Visible } else { Visibility::Hidden };
		if *visibility != wanted {
			*visibility = wanted;
		}
	}
	let Some(score) = score.filter(|_| shown) else {
		return;
	};
	let enemies = enemies.as_deref().copied();
	for (line, mut text, mut node) in &mut lines {
		let wanted = line.text(&score, enemies);
		let display = if wanted.is_some() { Display::Flex } else { Display::None };
		if node.display != display {
			node.display = display;
		}
		if let Some(wanted) = wanted.filter(|wanted| text.0 != *wanted) {
			text.0 = wanted;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn downs_build_a_streak_that_a_death_ends() {
		let mut score = CombatScore::default();
		score.record_down();
		score.record_down();
		score.record_death();
		score.record_down();
		assert_eq!((score.downs, score.deaths), (3, 1));
		assert_eq!((score.streak, score.best_streak), (1, 2));
		assert_eq!(score.headline(), "Downs  3");
		assert_eq!(score.tally(), "Downs  3    Deaths  1");
		assert_eq!(score.streaks(), "Streak  1    Best  2");
	}

	#[test]
	fn the_enemy_line_shows_only_while_a_count_is_kept() {
		let score = CombatScore::default();
		assert_eq!(CombatScoreLine::Enemies.text(&score, None), None);
		assert_eq!(
			CombatScoreLine::Enemies.text(&score, Some(LiveEnemies(7))).as_deref(),
			Some("Enemies  7")
		);
		assert!(CombatScoreLine::Tally.text(&score, None).is_some());
	}
}
