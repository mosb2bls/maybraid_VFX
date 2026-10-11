//! Flying signed point feedback that stacks above the Lubikan slot.

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use lubikans::{LubikanAccountId, LubikanChargeChanged, LubikanChargeReason, LubikanOwner};
use player::Player;

use crate::lubikan_charge::{LubikanChargeFlyTarget, LubikanPointFeedDock};
use crate::screen_pin::ScreenPin;
use crate::CombatHudVisible;

const FLY_DURATION: f32 = 0.55;
const STACK_HOLD: f32 = 2.0;
const SLOT_H: f32 = 22.0;
const MAX_FLYERS: usize = 24;
const EDGE_MARGIN: f32 = 28.0;
const GAIN: Color = Color::srgb(0.45, 0.95, 0.72);
const LOSS: Color = Color::srgb(0.95, 0.38, 0.32);

#[derive(Component)]
pub(crate) struct LubikanPointFlyer {
	credited: LubikanAccountId,
	born: f32,
	start: Vec2,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ingest_lubikan_point_flyers(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	players: Query<Entity, With<Player>>,
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	mut changes: MessageReader<LubikanChargeChanged>,
	hud_root: Query<Entity, With<crate::CombatHudRoot>>,
	mut commands: Commands,
	flyers: Query<Entity, With<LubikanPointFlyer>>,
) {
	if !hud.0 {
		changes.clear();
		return;
	}
	let Ok(player) = players.single() else {
		changes.clear();
		return;
	};
	let Ok(hud) = hud_root.single() else {
		changes.clear();
		return;
	};
	let Ok((camera, camera_transform)) = cameras.single() else {
		changes.clear();
		return;
	};
	let live = flyers.iter().count();
	let mut budget = MAX_FLYERS.saturating_sub(live);
	for change in changes.read() {
		if change.recipient != player || !change.amount.is_finite() || change.amount == 0.0 {
			continue;
		}
		if budget == 0 {
			break;
		}
		budget -= 1;
		let start = project_origin(camera, camera_transform, change.origin);
		spawn_flyer(
			&mut commands,
			hud,
			change.lubikan,
			change.amount,
			change.reason,
			start,
			time.elapsed_secs(),
		);
	}
}

fn project_origin(
	camera: &Camera,
	camera_transform: &GlobalTransform,
	origin: Option<Vec3>,
) -> Vec2 {
	let world = origin.unwrap_or(Vec3::ZERO);
	ScreenPin::project(camera, camera_transform, world, EDGE_MARGIN)
		.map(|(screen, _)| screen)
		.unwrap_or_else(|| {
			let rect = camera.logical_viewport_rect().unwrap_or_default();
			rect.center()
		})
}

fn spawn_flyer(
	commands: &mut Commands,
	hud: Entity,
	credited: LubikanAccountId,
	amount: f32,
	reason: LubikanChargeReason,
	start: Vec2,
	born: f32,
) {
	let label = point_label_with_reason(amount, reason);
	let color = if amount > 0.0 { GAIN } else { LOSS };
	commands.entity(hud).with_children(|root| {
		root.spawn((
			Name::new("lubikan-point-flyer"),
			LubikanPointFlyer { credited, born, start },
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(start.x),
				top: Val::Px(start.y),
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				..default()
			},
			Text::new(label),
			TextFont { font_size: FontSize::Px(18.0), ..default() },
			TextLayout::new(Justify::Left, LineBreak::NoWrap),
			TextColor(color),
			Pickable::IGNORE,
		));
	});
}

type PlayerWithLedger<'w, 's> =
	Query<'w, 's, (Entity, &'static lubikans::LubikanLedger), (With<Player>, With<LubikanOwner>)>;

pub(crate) fn update_lubikan_point_flyers(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	players: PlayerWithLedger,
	docks: Query<&UiGlobalTransform, With<LubikanPointFeedDock>>,
	anchors: Query<&UiGlobalTransform, With<LubikanChargeFlyTarget>>,
	mut flyers: Query<(Entity, &mut LubikanPointFlyer, &mut Node, &mut TextColor, &mut Visibility)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	let show = hud.0 && players.single().is_ok();
	let displayed_account = players.single().ok().map(|(_, ledger)| ledger.equipped().id());
	if !show {
		for (entity, _, _, _, _) in &mut flyers {
			commands.entity(entity).try_despawn();
		}
		return;
	}

	let dock = docks
		.iter()
		.next()
		.or_else(|| anchors.iter().next())
		.map(|tf| {
			let t = tf.affine().translation;
			Vec2::new(t.x, t.y)
		})
		.unwrap_or(Vec2::new(48.0, 48.0));

	let mut order: Vec<(Entity, f32)> =
		flyers.iter().map(|(entity, flyer, _, _, _)| (entity, flyer.born)).collect();
	order.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(std::cmp::Ordering::Equal));

	for (index, (entity, _)) in order.into_iter().enumerate() {
		let Ok((_, flyer, mut node, mut color, mut visibility)) = flyers.get_mut(entity) else {
			continue;
		};
		let age = now - flyer.born;
		if age >= FLY_DURATION + STACK_HOLD {
			commands.entity(entity).try_despawn();
			continue;
		}
		if Some(flyer.credited) != displayed_account {
			commands.entity(entity).try_despawn();
			continue;
		}
		let dest = dock + stack_offset(index);
		let t = (age / FLY_DURATION).clamp(0.0, 1.0);
		let eased = 1.0 - (1.0 - t).powi(2);
		let pos = flyer.start.lerp(dest, eased);
		node.left = Val::Px(pos.x);
		node.top = Val::Px(pos.y);
		color.0 = color.0.with_alpha(1.0);
		*visibility = Visibility::Visible;
	}
}

pub(crate) fn stack_offset(index: usize) -> Vec2 {
	Vec2::new(0.0, -((index as f32 + 1.0) * SLOT_H))
}

pub(crate) fn point_label_with_reason(amount: f32, reason: LubikanChargeReason) -> String {
	let signed = if amount > 0.0 {
		format!("+{:.0}", amount)
	} else {
		format!("{:.0}", amount)
	};
	format!("{signed} {label}", label = reason.label())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn signed_labels_distinguish_gains_and_penalties() {
		assert_eq!(
			point_label_with_reason(3.0, LubikanChargeReason::COMBAT_HIT),
			"+3 hit"
		);
		assert_eq!(
			point_label_with_reason(-2.0, LubikanChargeReason::ADJUSTMENT),
			"-2 adjustment"
		);
	}

	#[test]
	fn stack_slots_rise_above_the_dock() {
		assert_eq!(stack_offset(0), Vec2::new(0.0, -SLOT_H));
		assert_eq!(stack_offset(1), Vec2::new(0.0, -2.0 * SLOT_H));
		assert!(stack_offset(0).y > stack_offset(2).y);
		assert_eq!(stack_offset(0).x, 0.0, "feed stays left-justified over the plate");
	}

	#[test]
	fn flyers_linger_after_they_land() {
		assert!(FLY_DURATION + STACK_HOLD >= 2.0);
		assert!(STACK_HOLD >= 2.0);
	}
}
