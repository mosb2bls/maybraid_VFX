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

	#[test]
	fn flyers_retire_when_credited_account_changes() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_message::<lubikans::LubikanChargeChanged>();
		app.init_resource::<CombatHudVisible>();
		app.init_resource::<Time>();
		
		app.add_systems(bevy::app::Update, (ingest_lubikan_point_flyers, update_lubikan_point_flyers).chain());
		
		// Spawn HUD root
		let hud_root = app.world_mut().spawn((
			crate::CombatHudRoot,
			Node::default(),
		)).id();
		
		// Spawn charge fly target
		app.world_mut().spawn((
			LubikanChargeFlyTarget,
			Node::default(),
			bevy::ui::UiGlobalTransform::default(),
		));
		
		// Create two account IDs
		let account_a = LubikanAccountId::SELF;
		let account_b = LubikanAccountId::new(42);
		
		// Spawn player with ledger showing account_a as equipped
		let mut ledger = lubikans::LubikanLedger::with_self();
		ledger.apply_to_equipped(50.0);
		let player = app.world_mut().spawn((
			Player,
			LubikanOwner,
			ledger,
			bevy::transform::components::Transform::default(),
		)).id();
		
		// Spawn camera
		app.world_mut().spawn((
			bevy::render::camera::Camera3d::default(),
			bevy::transform::components::Transform::default(),
			bevy::transform::components::GlobalTransform::default(),
		));
		
		// Send charge change for account_a
		app.world_mut().write_message(lubikans::LubikanChargeChanged {
			recipient: player,
			lubikan: account_a,
			origin: Some(Vec3::ZERO),
			amount: 10.0,
			reason: LubikanChargeReason::COMBAT_HIT,
		});
		
		// Run one update to spawn the flyer
		app.update();
		
		// Verify flyer was spawned
		let flyer_count = app.world().query::<&LubikanPointFlyer>().iter(app.world()).count();
		assert_eq!(flyer_count, 1, "Should have spawned one flyer");
		
		// Verify flyer is credited to account_a
		let flyer_account = app.world().query::<&LubikanPointFlyer>()
			.iter(app.world())
			.next()
			.map(|f| f.credited);
		assert_eq!(flyer_account, Some(account_a), "Flyer should be credited to account_a");
		
		// Switch equipped account to account_b
		let mut ledger = app.world_mut().get_mut::<lubikans::LubikanLedger>(player).unwrap();
		// Create a second account and switch to it (simplified for test)
		ledger.register(account_b, lubikans::LubikanCatalogEntry {
			id: account_b,
			name: "Test",
			category: lubikans::LubikanCategory::COMBAT,
		});
		ledger.select(account_b);
		
		// Run update - flyer should be retired because credited account doesn't match displayed
		app.update();
		
		// Verify flyer was retired
		let flyer_count = app.world().query::<&LubikanPointFlyer>().iter(app.world()).count();
		assert_eq!(flyer_count, 0, "Flyer should be retired when credited account no longer displayed");
	}
}
