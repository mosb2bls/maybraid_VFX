//! Right-side loadout plate: weapon card above the skill-map viewport.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use menu_components::{HudFonts, TEXT_YELLOW};

use crate::user::SkillMapUser;
use crate::viewport::VIEWPORT_PX;

/// Chip / name size — the original objective face, not the title weight.
pub(crate) const CHROME_FONT: f32 = 16.0;
/// Prompt verbs sit a step under the name.
pub(crate) const PROMPT_FONT: f32 = 14.0;
const GLYPH_FONT: f32 = 11.0;
const PLATE_PAD: f32 = 12.0;
const PLATE_GAP: f32 = 8.0;
const PLATE_RADIUS: f32 = 14.0;
const INSET: f32 = 16.0;
const MAP_GAP: f32 = 8.0;
const GLYPH_PX: f32 = 22.0;

/// Lighter than the old charcoal so yellow type does not sink.
pub(crate) const WEAPON_FILL: Color = Color::srgb(0.46, 0.46, 0.48);

#[derive(Component)]
pub(crate) struct LoadoutHud {
	user: Entity,
}

#[derive(Component)]
pub(crate) struct LoadoutWeaponName;

/// RT Fire, then Y Switch — one row under the silhouette.
pub(crate) fn weapon_prompts() -> [(&'static str, &'static str); 2] {
	[("RT", "Fire"), ("Y", "Switch")]
}

/// RB Hold sits on the skill-map frame, not the weapon card.
pub(crate) fn map_hold_prompt() -> (&'static str, &'static str) {
	("RB", "Hold")
}

/// Stack the hashed name on word breaks so it matches the three-line plate.
pub(crate) fn stacked_weapon_name(name: &str) -> String {
	name.split_whitespace().collect::<Vec<_>>().join("\n")
}

fn plate_bottom() -> f32 {
	INSET + VIEWPORT_PX + MAP_GAP
}

pub(crate) fn present_loadout_hud(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	users: Query<(Entity, &InventoryUser), With<SkillMapUser>>,
	bags: Query<&Inventory>,
	plates: Query<(Entity, &LoadoutHud)>,
) {
	let live: Vec<Entity> = users.iter().map(|(entity, _)| entity).collect();
	for (entity, hud) in &plates {
		if !live.contains(&hud.user) {
			commands.entity(entity).try_despawn();
		}
	}
	let fonts = HudFonts::load(asset_server.as_ref());
	for (user, carrying) in &users {
		if plates.iter().any(|(_, hud)| hud.user == user) {
			continue;
		}
		let Ok(bag) = bags.get(carrying.bag) else {
			continue;
		};
		let Some(weapon) = bag.primary_weapon() else {
			continue;
		};
		spawn_weapon_plate(&mut commands, &fonts, user, &weapon.name());
	}
}

pub(crate) fn sync_loadout_weapon(
	users: Query<(Entity, &InventoryUser), With<SkillMapUser>>,
	bags: Query<&Inventory>,
	plates: Query<(&LoadoutHud, &Children)>,
	mut names: Query<&mut Text, With<LoadoutWeaponName>>,
) {
	for (hud, children) in &plates {
		let Ok((_, carrying)) = users.get(hud.user) else {
			continue;
		};
		let Ok(bag) = bags.get(carrying.bag) else {
			continue;
		};
		let Some(weapon) = bag.primary_weapon() else {
			continue;
		};
		let stacked = stacked_weapon_name(&weapon.name());
		for child in children {
			if let Ok(mut text) = names.get_mut(*child) {
				if text.0 != stacked {
					text.0 = stacked.clone();
				}
			}
		}
	}
}

fn spawn_weapon_plate(commands: &mut Commands, fonts: &HudFonts, user: Entity, name: &str) {
	commands
		.spawn((
			Name::new("loadout-weapon"),
			LoadoutHud { user },
			Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(plate_bottom()),
				right: Val::Px(INSET),
				width: Val::Px(VIEWPORT_PX),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(PLATE_GAP),
				padding: UiRect::all(Val::Px(PLATE_PAD)),
				border_radius: BorderRadius::all(Val::Px(PLATE_RADIUS)),
				..default()
			},
			BackgroundColor(WEAPON_FILL),
			Pickable::IGNORE,
		))
		.with_children(|plate| {
			plate.spawn((
				LoadoutWeaponName,
				Text::new(stacked_weapon_name(name)),
				fonts.item(CHROME_FONT),
				TextColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
			spawn_gun_mark(plate);
			plate
				.spawn((
					Node {
						flex_direction: FlexDirection::Row,
						justify_content: JustifyContent::SpaceBetween,
						align_items: AlignItems::Center,
						column_gap: Val::Px(12.0),
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|row| {
					for (glyph, verb) in weapon_prompts() {
						spawn_pad_prompt(row, fonts, glyph, verb);
					}
				});
		});
}

fn spawn_gun_mark(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			Node {
				height: Val::Px(36.0),
				width: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|slot| {
			slot.spawn((
				Node {
					flex_direction: FlexDirection::Row,
					align_items: AlignItems::Center,
					column_gap: Val::Px(2.0),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|gun| {
				gun.spawn((
					Node {
						width: Val::Px(18.0),
						height: Val::Px(10.0),
						border_radius: BorderRadius::all(Val::Px(2.0)),
						..default()
					},
					BackgroundColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
				gun.spawn((
					Node {
						width: Val::Px(72.0),
						height: Val::Px(6.0),
						border_radius: BorderRadius::all(Val::Px(2.0)),
						..default()
					},
					BackgroundColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
		});
}

pub(crate) fn spawn_pad_prompt(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	glyph: &str,
	verb: &str,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				column_gap: Val::Px(6.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			row.spawn((
				Node {
					width: Val::Px(GLYPH_PX),
					height: Val::Px(GLYPH_PX),
					min_width: Val::Px(GLYPH_PX),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					border: UiRect::all(Val::Px(1.5)),
					border_radius: BorderRadius::all(Val::Px(GLYPH_PX)),
					padding: UiRect::axes(Val::Px(4.0), Val::Px(0.0)),
					..default()
				},
				BorderColor::all(TEXT_YELLOW),
				Pickable::IGNORE,
			))
			.with_children(|chip| {
				chip.spawn((
					Text::new(glyph.to_string()),
					fonts.item(GLYPH_FONT),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			row.spawn((
				Text::new(verb.to_string()),
				fonts.item(PROMPT_FONT),
				TextColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
		});
}

pub(crate) fn spawn_caption_chip(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	caption: impl Into<String>,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Node {
				padding: UiRect::axes(Val::Px(8.0), Val::Px(1.0)),
				border: UiRect::all(Val::Px(1.5)),
				border_radius: BorderRadius::all(Val::Px(8.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				flex_shrink: 0.0,
				..default()
			},
			BorderColor::all(TEXT_YELLOW),
			BackgroundColor(TEXT_YELLOW.with_alpha(0.14)),
			Pickable::IGNORE,
			extra,
		))
		.with_children(|chip| {
			chip.spawn((
				Text::new(caption.into()),
				fonts.item(CHROME_FONT),
				TextColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
		});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn name_stacks_on_word_breaks() {
		assert_eq!(
			stacked_weapon_name("Brushed Midnight Hush Gun"),
			"Brushed\nMidnight\nHush\nGun"
		);
	}

	#[test]
	fn rt_and_y_sit_together_under_the_weapon() {
		assert_eq!(weapon_prompts(), [("RT", "Fire"), ("Y", "Switch")]);
	}

	#[test]
	fn rb_is_the_map_hold() {
		assert_eq!(map_hold_prompt(), ("RB", "Hold"));
	}

	#[test]
	fn chrome_is_the_small_face() {
		assert!(CHROME_FONT <= 16.0);
		assert!(PROMPT_FONT < CHROME_FONT);
	}

	#[test]
	fn weapon_card_is_a_light_gray() {
		let Color::Srgba(rgba) = WEAPON_FILL else {
			panic!("expected srgb fill");
		};
		assert!(rgba.red > 0.4, "fill should read lighter than charcoal");
	}
}
