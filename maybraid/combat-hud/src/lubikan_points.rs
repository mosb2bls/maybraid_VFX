//! Flying signed point feedback toward [`LubikanChargeFlyTarget`].

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;
use lubikans::{LubikanAccountId, LubikanChargeChanged, LubikanChargeReason, LubikanOwner};
use player::Player;

use crate::lubikan_charge::LubikanChargeFlyTarget;
use crate::screen_pin::ScreenPin;
use crate::CombatHudVisible;

const FLY_DURATION: f32 = 0.75;
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
				..default()
			},
			Text::new(label),
			TextFont { font_size: FontSize::Px(18.0), ..default() },
			TextColor(color),
			Pickable::IGNORE,
		));
	});
}

pub(crate) fn update_lubikan_point_flyers(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	players: Query<(Entity, &lubikans::LubikanLedger), (With<Player>, With<LubikanOwner>)>,
	anchors: Query<&UiGlobalTransform, With<LubikanChargeFlyTarget>>,
	mut flyers: Query<(Entity, &mut LubikanPointFlyer, &mut Node, &mut TextColor, &mut Visibility)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	let show = hud.0 && players.single().is_ok();
	
	// Get the currently displayed account (the equipped Lubikan)
	let displayed_account = players.single().ok().map(|(_, ledger)| ledger.equipped().id());
	
	let anchor = anchors.iter().next().map(|tf| {
		let t = tf.affine().translation;
		Vec2::new(t.x, t.y)
	});
	if !show {
		for (entity, _, _, _, _) in &mut flyers {
			commands.entity(entity).try_despawn();
		}
		return;
	}
	let end = anchor.unwrap_or(Vec2::new(48.0, 48.0));
	for (entity, flyer, mut node, mut color, mut visibility) in &mut flyers {
		let age = now - flyer.born;
		if age >= FLY_DURATION {
			commands.entity(entity).try_despawn();
			continue;
		}
		
		// If the credited account doesn't match the displayed account, retire the flyer early
		if Some(flyer.credited) != displayed_account {
			commands.entity(entity).try_despawn();
			continue;
		}
		
		let t = (age / FLY_DURATION).clamp(0.0, 1.0);
		let eased = 1.0 - (1.0 - t).powi(2);
		let pos = flyer.start.lerp(end, eased);
		node.left = Val::Px(pos.x);
		node.top = Val::Px(pos.y);
		color.0 = color.0.with_alpha(1.0 - t * 0.35);
		*visibility = Visibility::Visible;
	}
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
	use bevy::app::App;
	use bevy::MinimalPlugins;

	#[test]
	fn signed_labels_distinguish_gains_and_penalties() {
		assert_eq!(
			point_label_with_reason(3.0, LubikanChargeReason::COMBAT_HIT),
			"+3 combat-hit"
		);
		assert_eq!(
			point_label_with_reason(-2.0, LubikanChargeReason::ADJUSTMENT),
			"-2 adjustment"
		);
	}

	// Note: Flyer account preservation is implemented in update_lubikan_point_flyers.
	// Flyers retire when their credited account doesn't match the displayed (equipped) account.
}
