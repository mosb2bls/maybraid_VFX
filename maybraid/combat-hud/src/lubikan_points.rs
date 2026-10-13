//! Flying signed point feedback that stacks above the Lubikan slot.

use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
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
const MAX_GAIN: Color = Color::srgb(0.78, 0.72, 0.42);

#[derive(Component)]
pub(crate) struct LubikanPointFlyer {
	/// Account that earned this flyer. Kept across equipped-selection changes.
	#[allow(dead_code)]
	credited: LubikanAccountId,
	born: f32,
	start: Vec2,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn ingest_lubikan_point_flyers(
	time: Res<Time>,
	hud: Res<CombatHudVisible>,
	players: Query<Entity, With<Player>>,
	world_cameras: Query<(Entity, &Camera, &GlobalTransform), With<Camera3d>>,
	ui_cameras: Query<(Entity, &Camera, Has<IsDefaultUiCamera>)>,
	hud_targets: Query<&UiTargetCamera, With<crate::CombatHudRoot>>,
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
	let ui_entity = ui_cameras.iter().find_map(|(entity, _, is_ui)| is_ui.then_some(entity));
	let Some((camera, camera_transform)) = pick_world_camera(world_cameras.iter(), ui_entity)
	else {
		changes.clear();
		return;
	};
	let hud_target = hud_targets.iter().next().map(|UiTargetCamera(entity)| *entity);
	let ui_viewport = pick_ui_viewport(hud_target, ui_cameras.iter(), camera);
	let live = flyers.iter().count();
	let mut budget = MAX_FLYERS.saturating_sub(live);
	for change in changes.read() {
		if change.recipient != player
			|| !change.points_earned.is_finite()
			|| change.points_earned == 0.0
		{
			continue;
		}
		if budget == 0 {
			break;
		}
		budget -= 1;
		let start = project_origin(camera, camera_transform, ui_viewport, change.origin);
		let at_cap = change.discarded_at_stock_cap();
		spawn_flyer(
			&mut commands,
			hud,
			change.lubikan,
			point_label(change.points_earned, change.reason, at_cap),
			flyer_color(change.points_earned, at_cap),
			start,
			time.elapsed_secs(),
		);
	}
}

/// Gameplay `Camera3d` used for world projection. Prefer the camera that is
/// **not** the UI present camera (PR #1187 offscreen 3D + window blit).
fn pick_world_camera<'a>(
	world: impl Iterator<Item = (Entity, &'a Camera, &'a GlobalTransform)>,
	ui_entity: Option<Entity>,
) -> Option<(&'a Camera, &'a GlobalTransform)> {
	let mut fallback = None;
	let mut first = None;
	for (entity, camera, transform) in world {
		if first.is_none() {
			first = Some((camera, transform));
		}
		if Some(entity) == ui_entity {
			fallback = Some((camera, transform));
			continue;
		}
		if camera.is_active {
			return Some((camera, transform));
		}
	}
	fallback.or(first)
}

fn pick_ui_viewport<'a>(
	hud_target: Option<Entity>,
	cameras: impl Iterator<Item = (Entity, &'a Camera, bool)>,
	world_camera: &Camera,
) -> Option<Rect> {
	let mut default_ui = None;
	for (entity, camera, is_ui) in cameras {
		if Some(entity) == hud_target {
			if let Some(rect) = camera.logical_viewport_rect() {
				return Some(rect);
			}
		}
		if is_ui && default_ui.is_none() {
			default_ui = camera.logical_viewport_rect();
		}
	}
	default_ui.or_else(|| world_camera.logical_viewport_rect())
}

fn project_origin(
	camera: &Camera,
	camera_transform: &GlobalTransform,
	ui_viewport: Option<Rect>,
	origin: Option<Vec3>,
) -> Vec2 {
	let world = origin.unwrap_or(Vec3::ZERO);
	let projected =
		ScreenPin::project(camera, camera_transform, world, EDGE_MARGIN).map(|(screen, _)| screen);
	let world_viewport = camera.logical_viewport_rect();
	match (projected, world_viewport, ui_viewport) {
		(Some(screen), Some(from), Some(to)) => ScreenPin::map_between_viewports(screen, from, to),
		(Some(screen), _, _) => screen,
		(None, _, Some(ui)) => ui.center(),
		(None, Some(world_vp), _) => world_vp.center(),
		(None, None, None) => Vec2::ZERO,
	}
}

fn spawn_flyer(
	commands: &mut Commands,
	hud: Entity,
	credited: LubikanAccountId,
	label: String,
	color: Color,
	start: Vec2,
	born: f32,
) {
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
	docks: Query<(&UiGlobalTransform, &ComputedNode), With<LubikanPointFeedDock>>,
	anchors: Query<(&UiGlobalTransform, &ComputedNode), With<LubikanChargeFlyTarget>>,
	mut flyers: Query<(Entity, &mut LubikanPointFlyer, &mut Node, &mut TextColor, &mut Visibility)>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	let show = hud.0 && players.single().is_ok();
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
		.map(|(tf, computed)| ScreenPin::ui_logical_top_left(tf, computed))
		.filter(|pos| pos.is_finite())
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

pub(crate) fn point_label(amount: f32, reason: LubikanChargeReason, at_stock_cap: bool) -> String {
	let signed = if amount > 0.0 { format!("+{:.0}", amount) } else { format!("{:.0}", amount) };
	let base = format!("{signed} {label}", label = reason.label());
	if at_stock_cap && amount > 0.0 {
		format!("{base} MAX")
	} else {
		base
	}
}

pub(crate) fn flyer_color(amount: f32, at_stock_cap: bool) -> Color {
	if at_stock_cap && amount > 0.0 {
		MAX_GAIN
	} else if amount > 0.0 {
		GAIN
	} else {
		LOSS
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};
	use bevy::math::Affine2;

	#[test]
	fn signed_labels_distinguish_gains_and_penalties() {
		assert_eq!(point_label(3.0, LubikanChargeReason::COMBAT_HIT, false), "+3 hit");
		assert_eq!(point_label(-2.0, LubikanChargeReason::ADJUSTMENT, false), "-2 adjustment");
		assert_eq!(point_label(3.0, LubikanChargeReason::COMBAT_HIT, true), "+3 hit MAX");
		assert_eq!(flyer_color(3.0, true), MAX_GAIN);
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

	#[test]
	fn project_origin_uses_ui_center_when_the_3d_target_has_no_viewport() {
		let camera = Camera::default();
		let transform = GlobalTransform::default();
		let ui = Rect::from_corners(Vec2::ZERO, Vec2::new(1280.0, 720.0));
		let start = project_origin(&camera, &transform, Some(ui), Some(Vec3::new(4.0, 1.0, -8.0)));
		assert_eq!(start, ui.center(), "unready Image target must not spawn at (0,0)");
		assert!(start.x > 0.0 && start.x < 1280.0 && start.y > 0.0 && start.y < 720.0);
	}

	fn flyer_app() -> (App, Entity) {
		let mut app = App::new();
		lubikans::fixtures::install_interface(&mut app);
		app.add_plugins(MinimalPlugins).add_plugins(crate::CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: true,
			lubikan_charge: true,
			lubikan_point_feedback: true,
			score: false,
		});
		app.update();
		let owner = lubikans::fixtures::spawn_self_owner(app.world_mut());
		app.world_mut().entity_mut(owner).insert(Player);
		app.world_mut().spawn((
			Camera3d::default(),
			Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
			GlobalTransform::from(
				Transform::from_xyz(0.0, 2.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
			),
		));
		(app, owner)
	}

	fn node_px(node: &Node) -> Result<Vec2> {
		let Val::Px(left) = node.left else {
			return Err(anyhow!("left {:?}", node.left));
		};
		let Val::Px(top) = node.top else {
			return Err(anyhow!("top {:?}", node.top));
		};
		Ok(Vec2::new(left, top))
	}

	fn set_dock_physical(world: &mut World, center: Vec2, inverse_scale: f32) -> Result<()> {
		let mut docks = world
			.query_filtered::<(&mut UiGlobalTransform, &mut ComputedNode), With<LubikanPointFeedDock>>(
			);
		let (mut transform, mut computed) =
			docks.iter_mut(world).next().ok_or_else(|| anyhow!("charge-row dock"))?;
		*transform = UiGlobalTransform::from(Affine2::from_translation(center));
		computed.size = Vec2::new(2.0, 2.0);
		computed.inverse_scale_factor = inverse_scale;
		Ok(())
	}

	fn emit_hit(app: &mut App, owner: Entity, origin: Option<Vec3>) -> Result<()> {
		let account = app
			.world()
			.get::<lubikans::LubikanLedger>(owner)
			.ok_or_else(|| anyhow!("ledger"))?
			.equipped()
			.id();
		app.world_mut().write_message(LubikanChargeChanged::new(
			owner,
			account,
			3.0,
			LubikanChargeReason::COMBAT_HIT,
			origin,
		));
		Ok(())
	}

	fn emit_capped_hit(app: &mut App, owner: Entity, origin: Option<Vec3>) -> Result<()> {
		let account = app
			.world()
			.get::<lubikans::LubikanLedger>(owner)
			.ok_or_else(|| anyhow!("ledger"))?
			.equipped()
			.id();
		app.world_mut().write_message(
			LubikanChargeChanged::new(owner, account, 0.0, LubikanChargeReason::COMBAT_HIT, origin)
				.with_points_earned(3.0),
		);
		Ok(())
	}

	fn land_flyers(app: &mut App) {
		let now = app.world().resource::<Time>().elapsed_secs();
		let mut flyers = app.world_mut().query::<&mut LubikanPointFlyer>();
		for mut flyer in flyers.iter_mut(app.world_mut()) {
			flyer.born = now - FLY_DURATION;
		}
		app.update();
	}

	fn flyer_nodes(world: &mut World) -> Result<Vec<Vec2>> {
		world
			.query_filtered::<&Node, With<LubikanPointFlyer>>()
			.iter(world)
			.map(node_px)
			.collect()
	}

	/// PR #1187 present-camera / Retina case: dock `UiGlobalTransform` is
	/// physical (2×), window logical is 1280×720. Old dest used the raw
	/// translation as `Node::top` and flew to y≈1178 (off-screen).
	#[test]
	fn flyers_land_at_logical_charge_anchor_on_hidpi_ui_camera() -> Result<()> {
		let (mut app, owner) = flyer_app();
		app.update();
		set_dock_physical(app.world_mut(), Vec2::new(96.0, 1200.0), 0.5)?;
		emit_hit(&mut app, owner, Some(Vec3::new(2.0, 1.0, -6.0)))?;
		app.update();
		assert_eq!(
			app.world_mut().query::<&LubikanPointFlyer>().iter(app.world()).count(),
			1,
			"charge change must spawn a flyer"
		);
		land_flyers(&mut app);
		let landed = flyer_nodes(app.world_mut())?;
		assert_eq!(landed.len(), 1);
		let dest = ScreenPin::ui_logical_top_left(
			&UiGlobalTransform::from(Affine2::from_translation(Vec2::new(96.0, 1200.0))),
			&ComputedNode { size: Vec2::new(2.0, 2.0), inverse_scale_factor: 0.5, ..default() },
		) + stack_offset(0);
		assert!((landed[0] - dest).length() < 1.0, "landed {:?} dest {:?}", landed[0], dest);
		assert!(
			landed[0].x >= 0.0
				&& landed[0].x <= 1280.0
				&& landed[0].y >= 0.0
				&& landed[0].y <= 720.0,
			"flyer ended off-screen at {:?}",
			landed[0]
		);
		assert!(landed[0].y < 720.0, "physical dock Y must not be written to Node::top");
		Ok(())
	}

	#[test]
	fn flyers_stack_above_the_logical_dock() -> Result<()> {
		let (mut app, owner) = flyer_app();
		app.update();
		set_dock_physical(app.world_mut(), Vec2::new(96.0, 1200.0), 0.5)?;
		emit_hit(&mut app, owner, Some(Vec3::new(2.0, 1.0, -6.0)))?;
		emit_hit(&mut app, owner, Some(Vec3::new(3.0, 1.0, -5.0)))?;
		app.update();
		assert_eq!(app.world_mut().query::<&LubikanPointFlyer>().iter(app.world()).count(), 2);
		land_flyers(&mut app);
		let mut landed = flyer_nodes(app.world_mut())?;
		landed.sort_by(|left, right| {
			left.y.partial_cmp(&right.y).unwrap_or(std::cmp::Ordering::Equal)
		});
		assert!((landed[1].y - landed[0].y - SLOT_H).abs() < 1.0, "{landed:?}");
		assert!((landed[0].x - landed[1].x).abs() < 1.0, "stack stays left-justified");
		assert!(landed.iter().all(|pos| pos.y >= 0.0 && pos.y <= 720.0), "{landed:?}");
		Ok(())
	}

	/// #1187-style split: 3D camera is not the UI camera. Ingest must still
	/// spawn (it used to require `Query<..., With<Camera3d>>::single()`).
	#[test]
	fn flyers_spawn_when_ui_lives_on_a_present_camera() -> Result<()> {
		let (mut app, owner) = flyer_app();
		app.world_mut()
			.spawn((Camera2d, Camera { order: 1, ..default() }, IsDefaultUiCamera));
		app.update();
		set_dock_physical(app.world_mut(), Vec2::new(80.0, 600.0), 1.0)?;
		emit_hit(&mut app, owner, Some(Vec3::new(1.0, 1.0, -4.0)))?;
		app.update();
		assert_eq!(
			app.world_mut().query::<&LubikanPointFlyer>().iter(app.world()).count(),
			1,
			"present Camera2d must not starve Camera3d::single projection"
		);
		Ok(())
	}

	#[test]
	fn flyers_still_spawn_when_charge_is_already_full() -> Result<()> {
		let (mut app, owner) = flyer_app();
		app.update();
		set_dock_physical(app.world_mut(), Vec2::new(80.0, 600.0), 1.0)?;
		emit_capped_hit(&mut app, owner, Some(Vec3::new(1.0, 1.0, -4.0)))?;
		app.update();
		let world = app.world_mut();
		assert_eq!(
			world.query::<&LubikanPointFlyer>().iter(world).count(),
			1,
			"capped awards must still raise a flyer"
		);
		assert!(
			world.query::<&Text>().iter(world).any(|text| text.0 == "+3 hit MAX"),
			"full-state cue must keep the earned-point label"
		);
		Ok(())
	}
}
