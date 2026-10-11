//! Active Lubikan charge plate stacked above player vitals.

use bevy::prelude::*;
use lubikans::{
	LubikanAccountId, LubikanActivation, LubikanActivationFinished, LubikanActivationOutcome,
	LubikanActivationPhase, LubikanCatalog, LubikanCharge, LubikanLedger, LubikanOwner,
	LubikanReadiness, LubikanSelfHeal, LubikanSelfHealChannel,
};
use menu_components::{spawn_hud_text_card, HudFonts, HUD_TEXT_CARD_FACE_PX, TEXT_YELLOW};
use player::Player;

use crate::continuous_health::{spawn_ui_meter_row, ContinuousHealthConfig};
use crate::cycle_cue::CombatHudInventoryCycleCategory;
use crate::plate::{spawn_hud_plate, HudPlateCorner, HudPlateLayout};
use crate::vitals::VITALS_WIDTH;
use crate::CombatHudVisible;

/// Vertical space reserved for the vitals plate so charge sits above it.
pub(crate) const VITALS_STACK_OFFSET: f32 = 108.0;

const CHARGE_WIDTH: f32 = VITALS_WIDTH;
const CHARGE_ROW_GAP: f32 = 6.0;
const CROSS_PX: f32 = 22.0;
const CROSS_ARM: f32 = 6.0;
const READY_GLOW: Color = Color::srgba(0.45, 0.95, 0.72, 0.35);
const CATEGORY_CUE: Color = Color::srgb(0.55, 0.82, 1.0);
const BAR_READY: Color = Color::srgb(0.35, 0.92, 0.62);
const BAR_LOW: Color = Color::srgb(0.86, 0.28, 0.22);
const BAR_MID: Color = Color::srgb(0.95, 0.72, 0.22);
const BAR_FULL: Color = TEXT_YELLOW;
const MUTED: Color = Color::srgb(0.72, 0.74, 0.78);
const ACTIVATION_FINISHED_PULSE_SECS: f32 = 1.25;

/// Brief status from [`LubikanActivationFinished`] (read-only; not ledger phase).
#[derive(Resource, Default)]
pub(crate) struct LubikanActivationFinishedHudPulse {
	until: f32,
	outcome: Option<LubikanActivationOutcome>,
}

impl LubikanActivationFinishedHudPulse {
	pub(crate) fn record(&mut self, at: f32, outcome: LubikanActivationOutcome) {
		self.until = at + ACTIVATION_FINISHED_PULSE_SECS;
		self.outcome = Some(outcome);
	}

	pub(crate) fn clear(&mut self) {
		self.until = 0.0;
		self.outcome = None;
	}

	pub(crate) fn active_label(&self, now: f32) -> Option<&'static str> {
		if now >= self.until {
			return None;
		}
		match self.outcome {
			Some(LubikanActivationOutcome::Completed) => Some("completed"),
			Some(LubikanActivationOutcome::Cancelled) => Some("cancelled"),
			None => None,
		}
	}
}

#[allow(clippy::type_complexity)]
pub(crate) fn ingest_lubikan_activation_finished_pulse(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	players: Query<(Entity, &LubikanLedger), (With<Player>, With<LubikanOwner>)>,
	mut finished: MessageReader<LubikanActivationFinished>,
	mut pulse: ResMut<LubikanActivationFinishedHudPulse>,
) {
	if !hud.0 {
		finished.clear();
		pulse.clear();
		return;
	}
	let Some((owner, ledger)) = players.iter().next() else {
		finished.clear();
		return;
	};
	let equipped = ledger.equipped().id();
	let now = time.elapsed_secs();
	for event in finished.read() {
		if event.owner == owner && event.account == equipped {
			pulse.record(now, event.outcome);
		}
	}
}

#[derive(Component)]
pub(crate) struct LubikanChargeRoot;

/// Stable screen-space target for flying point feedback toward this Lubikan row.
#[derive(Component)]
pub struct LubikanChargeFlyTarget;

#[derive(Component)]
pub(crate) struct LubikanChargeAmount;

#[derive(Component)]
pub(crate) struct LubikanChargeName;

#[derive(Component)]
pub(crate) struct LubikanChargeStatus;

/// Dock above the plate where point flyers stack.
#[derive(Component)]
pub(crate) struct LubikanPointFeedDock;

#[derive(Component)]
pub(crate) struct LubikanChargeFill;

#[derive(Component)]
pub(crate) struct LubikanChargeIcon;

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

pub(crate) fn spawn_lubikan_charge(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	config: &ContinuousHealthConfig,
) {
	spawn_hud_plate(
		parent,
		"lubikan-charge",
		LubikanChargeRoot,
		HudPlateLayout {
			width: CHARGE_WIDTH,
			row_gap: CHARGE_ROW_GAP,
			corner: HudPlateCorner::BottomLeft,
			bottom_lift: VITALS_STACK_OFFSET,
		},
		|plate| {
			plate.spawn((
				LubikanPointFeedDock,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(-8.0),
					width: Val::Px(1.0),
					height: Val::Px(1.0),
					..default()
				},
				Pickable::IGNORE,
			));
			spawn_self_icon(plate);
			spawn_ui_meter_row(plate, config, LubikanChargeFill, LubikanChargeAmount);
			plate.spawn((
				LubikanChargeFlyTarget,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Px(1.0),
					height: Val::Px(1.0),
					..default()
				},
				Pickable::IGNORE,
			));
			spawn_hud_text_card(plate, (), |card| {
				card.spawn((
					LubikanChargeName,
					Text::new("Self"),
					fonts.item(HUD_TEXT_CARD_FACE_PX),
					TextColor(TEXT_YELLOW),
					Pickable::IGNORE,
				));
			});
			plate.spawn((
				LubikanChargeStatus,
				Text::new(""),
				fonts.body(12.0),
				TextColor(MUTED),
				Visibility::Hidden,
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
		},
	);
}

fn spawn_self_icon(parent: &mut ChildSpawnerCommands) {
	parent
		.spawn((
			LubikanChargeIcon,
			Node {
				width: Val::Px(CROSS_PX),
				height: Val::Px(CROSS_PX),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|plus| {
			plus.spawn((
				Node {
					width: Val::Px(CROSS_PX),
					height: Val::Px(CROSS_ARM),
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px((CROSS_PX - CROSS_ARM) * 0.5),
					border_radius: BorderRadius::all(Val::Px(1.0)),
					..default()
				},
				BackgroundColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
			plus.spawn((
				Node {
					width: Val::Px(CROSS_ARM),
					height: Val::Px(CROSS_PX),
					position_type: PositionType::Absolute,
					left: Val::Px((CROSS_PX - CROSS_ARM) * 0.5),
					top: Val::Px(0.0),
					border_radius: BorderRadius::all(Val::Px(1.0)),
					..default()
				},
				BackgroundColor(TEXT_YELLOW),
				Pickable::IGNORE,
			));
		});
}

/// Read-only Self heal presentation from [`LubikanSelfHeal`] / [`LubikanSelfHealChannel`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LubikanHudSelfHeal {
	pub remaining: f32,
	pub rate: f32,
}

pub(crate) fn self_heal_for_owner(
	owner: Entity,
	global: Option<&LubikanSelfHeal>,
	channel: Option<&LubikanSelfHealChannel>,
) -> Option<LubikanHudSelfHeal> {
	if let Some(channel) = channel.filter(|channel| channel.active) {
		return Some(LubikanHudSelfHeal { remaining: channel.remaining, rate: channel.rate });
	}
	global
		.filter(|heal| heal.active && heal.owner == Some(owner))
		.map(|heal| LubikanHudSelfHeal { remaining: heal.remaining, rate: heal.rate })
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn sync_lubikan_charge(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	catalog: Option<Res<LubikanCatalog>>,
	category: Option<Res<CombatHudInventoryCycleCategory>>,
	self_heal: Option<Res<LubikanSelfHeal>>,
	pulse: Option<Res<LubikanActivationFinishedHudPulse>>,
	players: Query<(Entity, &LubikanLedger), (With<Player>, With<LubikanOwner>)>,
	heal_channels: Query<&LubikanSelfHealChannel>,
	mut widgets: ParamSet<(
		Query<(&mut Visibility, &mut BorderColor), With<LubikanChargeRoot>>,
		Query<&mut Text, With<LubikanChargeAmount>>,
		Query<(&mut Text, &mut Visibility), With<LubikanChargeStatus>>,
		Query<(&mut Node, &mut BackgroundColor), With<LubikanChargeFill>>,
		Query<&mut Visibility, With<LubikanCategoryCue>>,
		Query<&mut Text, With<LubikanChargeName>>,
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
	let Some((owner, ledger)) = players.iter().next() else {
		hide();
		return;
	};
	let row = LubikanHudRow::from_ledger(ledger, catalog.as_ref());
	let heal = self_heal_for_owner(
		owner,
		self_heal.as_deref(),
		heal_channels.get(owner).ok(),
	);
	let finished = pulse.as_deref().and_then(|pulse| pulse.active_label(time.elapsed_secs()));
	let amount = format_charge(row.charge.current());
	let name = account_label(row.account);
	let status = charge_status(&row, heal, finished);
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
		if text.0 != amount {
			text.0 = amount.clone();
		}
	}
	for (mut text, mut visibility) in widgets.p2().iter_mut() {
		if text.0 != status {
			text.0 = status.clone();
		}
		*visibility = if status.is_empty() {
			Visibility::Hidden
		} else {
			Visibility::Visible
		};
	}
	for (mut node, mut color) in widgets.p3().iter_mut() {
		node.width = Val::Percent(fraction * 100.0);
		color.0 = fill_color;
	}
	let lubikan_selected = category.as_deref().is_some_and(|cat| cat.is_lubikans());
	for mut visibility in widgets.p4().iter_mut() {
		*visibility = if lubikan_selected { Visibility::Visible } else { Visibility::Hidden };
	}
	for mut text in widgets.p5().iter_mut() {
		if text.0 != name {
			text.0 = name.clone();
		}
	}
}

pub(crate) fn charge_status(
	row: &LubikanHudRow,
	heal: Option<LubikanHudSelfHeal>,
	finished: Option<&str>,
) -> String {
	if let Some(label) = finished {
		return label.to_string();
	}
	if let Some(heal) = heal.filter(|heal| row.account.is_self() && heal.remaining > 0.0) {
		if heal.rate > 0.0 {
			return format!(
				"Healing {:.0} ({:.0}/s)",
				heal.remaining.ceil(),
				heal.rate
			);
		}
		return format!("Healing {:.0}", heal.remaining.ceil());
	}
	match row.activation.phase {
		LubikanActivationPhase::Active if row.activation.session.is_some() => {
			String::from("Activating")
		}
		LubikanActivationPhase::Active => String::from("Active"),
		_ => String::new(),
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
	fn idle_status_stays_empty_so_the_bar_is_the_charge_readout() {
		let ready = LubikanHudRow {
			account: LubikanAccountId::SELF,
			charge: LubikanCharge::new(30.0, 100.0),
			readiness: LubikanReadiness::from_charge(LubikanAccountId::SELF, LubikanCharge::new(30.0, 100.0), 25.0, false),
			activation: LubikanActivation::idle(LubikanAccountId::SELF),
		};
		assert_eq!(charge_status(&ready, None, None), "");
		let healing = LubikanHudSelfHeal { remaining: 55.0, rate: 5.0 };
		assert_eq!(charge_status(&ready, Some(healing), None), "Healing 55 (5/s)");
		assert_eq!(charge_status(&ready, Some(healing), Some("completed")), "completed");
		let empty = LubikanHudRow {
			charge: LubikanCharge::new(3.0, 100.0),
			readiness: LubikanReadiness::from_charge(LubikanAccountId::SELF, LubikanCharge::new(3.0, 100.0), 25.0, false),
			..ready
		};
		assert_eq!(charge_status(&empty, None, None), "");
	}

	#[test]
	fn charge_amount_is_current_not_a_fraction() {
		assert_eq!(format_charge(3.0), "3");
		assert_eq!(format_charge(12.5), "12.5");
	}

	#[test]
	fn self_account_label_is_self() {
		assert_eq!(account_label(LubikanAccountId::SELF), "Self");
	}
}
