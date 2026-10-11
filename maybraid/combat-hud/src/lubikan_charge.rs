//! Active Lubikan charge plate stacked above player vitals.

use bevy::prelude::*;
use lubikans::{
	LubikanAccountId, LubikanActivation, LubikanActivationPhase, LubikanCatalog, LubikanCharge,
	LubikanLedger, LubikanOwner, LubikanReadiness,
};
use menu_components::{
	spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW,
};
use player::Player;

use crate::cycle_cue::CombatHudInventoryCycleCategory;
use crate::plate::HUD_CORNER_INSET;
use crate::vitals::VITALS_WIDTH;
use crate::CombatHudVisible;

/// **Placeholder ([#1110](https://github.com/ramate-io/maybraid/issues/1110)):** Self
/// healing presentation until the healing lane publishes active / remaining / rate.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct LubikanSelfHealingHudPlaceholder {
	pub active: bool,
	pub remaining_secs: f32,
}

/// Vertical space reserved for the vitals plate so charge sits above it.
pub(crate) const VITALS_STACK_OFFSET: f32 = 92.0;

const CHARGE_WIDTH: f32 = VITALS_WIDTH;
const CHARGE_BAR_HEIGHT: f32 = 12.0;
const CHARGE_ROW_GAP: f32 = 4.0;
const READY_GLOW: Color = Color::srgba(0.45, 0.95, 0.72, 0.35);
const CATEGORY_CUE: Color = Color::srgb(0.55, 0.82, 1.0);
const TRACK_FILL: Color = Color::srgb(0.10, 0.11, 0.13);
const TRACK_STROKE: Color = Color::srgba(0.08, 0.08, 0.10, 0.95);
const BAR_READY: Color = Color::srgb(0.35, 0.92, 0.62);
const BAR_LOW: Color = Color::srgb(0.86, 0.28, 0.22);
const BAR_MID: Color = Color::srgb(0.95, 0.72, 0.22);
const BAR_FULL: Color = TEXT_YELLOW;
const MUTED: Color = Color::srgb(0.72, 0.74, 0.78);

#[derive(Component)]
pub(crate) struct LubikanChargeRoot;

/// Stable screen-space target for flying point feedback toward this Lubikan row.
#[derive(Component)]
pub struct LubikanChargeFlyTarget;

#[derive(Component)]
pub(crate) struct LubikanChargeTitle;

#[derive(Component)]
pub(crate) struct LubikanChargeStatus;

#[derive(Component)]
pub(crate) struct LubikanChargeFill;

#[derive(Component)]
pub(crate) struct LubikanCategoryCue;

/// Read-only snapshot for presentation and tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LubikanHudRow {
	pub account: LubikanAccountId,
	pub charge: LubikanCharge,
	pub readiness: LubikanReadiness,
	pub activation: LubikanActivation,
}

impl LubikanHudRow {
	pub fn from_ledger(ledger: &LubikanLedger, catalog: &LubikanCatalog) -> Self {
		let equipped = ledger.equipped();
		Self {
			account: equipped.id(),
			charge: equipped.charge(),
			readiness: ledger.readiness(catalog),
			activation: equipped.activation(),
		}
	}
}

pub(crate) fn spawn_lubikan_charge(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Name::new("lubikan-charge"),
			LubikanChargeRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(HUD_CORNER_INSET),
				bottom: Val::Px(HUD_CORNER_INSET + VITALS_STACK_OFFSET),
				width: Val::Px(CHARGE_WIDTH),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(CHARGE_ROW_GAP),
				padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
				border: UiRect::all(Val::Px(1.5)),
				border_radius: BorderRadius::all(Val::Px(8.0)),
				..default()
			},
			BackgroundColor(Color::srgba(0.52, 0.52, 0.56, 0.82)),
			BorderColor::all(Color::srgba(0.78, 0.78, 0.82, 0.9)),
			Visibility::Hidden,
			Pickable::IGNORE,
		))
		.with_children(|plate| {
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					LubikanChargeTitle,
					Text::new("Self  0/100"),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			plate
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(CHARGE_BAR_HEIGHT + 6.0),
						padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
						border: UiRect::all(Val::Px(1.0)),
						border_radius: BorderRadius::all(Val::Px(4.0)),
						..default()
					},
					BackgroundColor(TRACK_FILL),
					BorderColor::all(TRACK_STROKE),
					Pickable::IGNORE,
				))
				.with_children(|track| {
					track.spawn((
						LubikanChargeFill,
						Node {
							width: Val::Percent(0.0),
							height: Val::Percent(100.0),
							border_radius: BorderRadius::all(Val::Px(2.0)),
							..default()
						},
						BackgroundColor(BAR_MID),
						Pickable::IGNORE,
					));
					track.spawn((
						LubikanChargeFlyTarget,
						Node {
							position_type: PositionType::Absolute,
							right: Val::Px(2.0),
							top: Val::Px(0.0),
							width: Val::Px(4.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				});
			plate.spawn((
				LubikanChargeStatus,
				Text::new(""),
				fonts.body(12.0),
				TextColor(MUTED),
				Pickable::IGNORE,
			));
			plate.spawn((
				LubikanCategoryCue,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					bottom: Val::Px(0.0),
					width: Val::Px(3.0),
					border_radius: BorderRadius::all(Val::Px(2.0)),
					..default()
				},
				BackgroundColor(CATEGORY_CUE),
				Visibility::Hidden,
				Pickable::IGNORE,
			));
		});
}

#[allow(clippy::type_complexity)]
pub(crate) fn sync_lubikan_charge(
	hud: Res<CombatHudVisible>,
	catalog: Option<Res<LubikanCatalog>>,
	category: Option<Res<CombatHudInventoryCycleCategory>>,
	healing: Option<Res<LubikanSelfHealingHudPlaceholder>>,
	players: Query<&LubikanLedger, (With<Player>, With<LubikanOwner>)>,
	mut widgets: ParamSet<(
		Query<(&mut Visibility, &mut BorderColor), With<LubikanChargeRoot>>,
		Query<&mut Text, With<LubikanChargeTitle>>,
		Query<&mut Text, With<LubikanChargeStatus>>,
		Query<(&mut Node, &mut BackgroundColor), With<LubikanChargeFill>>,
		Query<&mut Visibility, With<LubikanCategoryCue>>,
	)>,
) {
	let mut hide = || {
		for (mut visibility, _) in widgets.p0().iter_mut() {
			*visibility = Visibility::Hidden;
		}
	};
	if !hud.0 {
		hide();
		return;
	}
	let Some(catalog) = catalog else {
		hide();
		return;
	};
	let Some(ledger) = players.iter().next() else {
		hide();
		return;
	};
	let row = LubikanHudRow::from_ledger(ledger, catalog.as_ref());
	let headline = charge_title(&row);
	let status = charge_status(&row, healing.as_deref());
	let fraction = row.charge.fraction();
	let fill_color = charge_bar_color(&row);
	let ready = row.readiness.can_activate;
	for (mut visibility, mut border) in widgets.p0().iter_mut() {
		*visibility = Visibility::Visible;
		*border = BorderColor::all(if ready {
			READY_GLOW
		} else {
			Color::srgba(0.78, 0.78, 0.82, 0.9)
		});
	}
	for mut text in widgets.p1().iter_mut() {
		if text.0 != headline {
			text.0 = headline.clone();
		}
	}
	for mut text in widgets.p2().iter_mut() {
		if text.0 != status {
			text.0 = status.clone();
		}
	}
	for (mut node, mut color) in widgets.p3().iter_mut() {
		node.width = Val::Percent(fraction * 100.0);
		color.0 = fill_color;
	}
	let lubikan_selected = category.as_deref().is_some_and(|cat| cat.is_lubikans());
	for mut visibility in widgets.p4().iter_mut() {
		*visibility = if lubikan_selected { Visibility::Visible } else { Visibility::Hidden };
	}
}

pub(crate) fn charge_title(row: &LubikanHudRow) -> String {
	let name = account_label(row.account);
	let current = format_charge(row.charge.current());
	let capacity = format_charge(row.charge.capacity());
	format!("{name}  {current}/{capacity}")
}

pub(crate) fn charge_status(
	row: &LubikanHudRow,
	healing: Option<&LubikanSelfHealingHudPlaceholder>,
) -> String {
	if let Some(healing) = healing.filter(|healing| {
		row.account.is_self() && healing.active && healing.remaining_secs > 0.0
	}) {
		return format!("Healing {:.0}s", healing.remaining_secs.ceil());
	}
	match row.activation.phase {
		LubikanActivationPhase::Active if row.activation.session.is_some() => {
			String::from("Activating")
		}
		LubikanActivationPhase::Active => String::from("Active"),
		LubikanActivationPhase::Completed => String::from("Completing"),
		LubikanActivationPhase::Idle if row.charge.current() <= 0.0 => String::from("Empty"),
		LubikanActivationPhase::Idle if row.readiness.can_activate => String::from("Ready"),
		LubikanActivationPhase::Idle => {
			let need = (row.readiness.activation_cost - row.charge.current()).max(0.0);
			format!("Need {:.0} more", need.ceil())
		}
	}
}

pub(crate) fn charge_bar_color(row: &LubikanHudRow) -> Color {
	let fraction = row.charge.fraction();
	if fraction <= 0.0 {
		BAR_LOW
	} else if row.readiness.can_activate {
		BAR_READY
	} else if fraction < 0.35 {
		BAR_LOW
	} else if fraction < 0.7 {
		BAR_MID
	} else {
		BAR_FULL
	}
}

fn account_label(account: LubikanAccountId) -> String {
	account
		.definition()
		.well_known_label()
		.map(str::to_string)
		.unwrap_or_else(|| format!("Lubikan {}", account.definition().raw()))
}

fn format_charge(value: f32) -> String {
	if value.fract().abs() < 0.05 {
		format!("{:.0}", value)
	} else {
		format!("{:.1}", value)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use lubikans::LubikanAccountId;

	#[test]
	fn ready_and_empty_status_lines() {
		let ready = LubikanHudRow {
			account: LubikanAccountId::SELF,
			charge: LubikanCharge::new(30.0, 100.0),
			readiness: LubikanReadiness::from_charge(LubikanAccountId::SELF, LubikanCharge::new(30.0, 100.0), 25.0, false),
			activation: LubikanActivation::idle(LubikanAccountId::SELF),
		};
		assert_eq!(charge_status(&ready, None), "Ready");
		let empty = LubikanHudRow {
			charge: LubikanCharge::new(0.0, 100.0),
			..ready
		};
		assert_eq!(charge_status(&empty, None), "Empty");
	}

	#[test]
	fn charge_title_includes_capacity() {
		let account = LubikanAccountId::unique(lubikans::LubikanDefinitionId::from_raw(7));
		let row = LubikanHudRow {
			account,
			charge: LubikanCharge::new(12.0, 50.0),
			readiness: LubikanReadiness::from_charge(account, LubikanCharge::new(12.0, 50.0), 10.0, false),
			activation: LubikanActivation::idle(account),
		};
		assert_eq!(charge_title(&row), "Lubikan 7  12/50");
	}
}
