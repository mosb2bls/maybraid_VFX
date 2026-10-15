//! Visible scrollbar for overflowing HUD panes.

use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

use crate::controls::hud_menu::{HudMenu, HudMenuItem};
use crate::modal::MenuModalStack;
use crate::theme::{SCROLLBAR_THUMB, SCROLLBAR_TRACK, SCROLLBAR_WIDTH, TILE_FOCUS_PAD};
use maybraid_input::{MenuNav, MenuNavImpulse};

const SCROLL_LINE_PX: f32 = 14.0;
const MIN_THUMB_PX: f32 = 24.0;
/// Pad / D-pad step when a scroll pane has no focusable HUD items (Loadout).
const HUD_NAV_SCROLL_LINE: f32 = 56.0;

/// Scrollable column that a [`HudScrollThumb`] tracks.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudScrollViewport;

/// Vertical track; hidden when content fits.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudScrollTrack;

/// Thumb whose height and offset follow the viewport.
#[derive(Component, Debug, Clone, Copy)]
pub struct HudScrollThumb {
	pub viewport: Entity,
}

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct HudScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

/// Row: growing scroll viewport plus a thin track.
pub fn spawn_scroll_pane(
	parent: &mut ChildSpawnerCommands,
	viewport_extra: impl Bundle,
	align: AlignItems,
	row_gap: f32,
) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(8.0),
				min_height: Val::Px(0.0),
				flex_grow: 1.0,
				flex_shrink: 1.0,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			viewport = row
				.spawn((
					HudScrollViewport,
					viewport_extra,
					Node {
						width: Val::Percent(100.0),
						flex_grow: 1.0,
						flex_shrink: 1.0,
						min_height: Val::Px(0.0),
						min_width: Val::Px(0.0),
						flex_direction: FlexDirection::Column,
						align_items: align,
						row_gap: Val::Px(row_gap),
						overflow: Overflow::scroll_y(),
						scrollbar_width: SCROLLBAR_WIDTH,
						..default()
					},
					ScrollPosition::default(),
					Pickable::default(),
				))
				.id();
			row.spawn((
				HudScrollTrack,
				Node {
					width: Val::Px(SCROLLBAR_WIDTH),
					height: Val::Percent(100.0),
					flex_shrink: 0.0,
					position_type: PositionType::Relative,
					..default()
				},
				BackgroundColor(SCROLLBAR_TRACK),
				Visibility::Hidden,
				Pickable::IGNORE,
			))
			.with_children(|track| {
				track.spawn((
					HudScrollThumb { viewport },
					Node {
						position_type: PositionType::Absolute,
						left: Val::Px(0.0),
						width: Val::Px(SCROLLBAR_WIDTH),
						height: Val::Px(MIN_THUMB_PX),
						top: Val::Px(0.0),
						..default()
					},
					BackgroundColor(SCROLLBAR_THUMB),
					Pickable::IGNORE,
				));
			});
		});
	viewport
}

/// Show the track only when the viewport overflows; size the thumb to the visible ratio.
pub fn sync_hud_scrollbars(
	viewports: Query<(&ScrollPosition, &ComputedNode), With<HudScrollViewport>>,
	mut tracks: Query<(&mut Visibility, &ComputedNode, &Children), With<HudScrollTrack>>,
	mut thumbs: Query<(&HudScrollThumb, &mut Node), Without<HudScrollTrack>>,
) {
	for (mut track_visibility, track_computed, children) in &mut tracks {
		let Some((thumb_entity, thumb)) = children
			.iter()
			.find_map(|child| thumbs.get(child).ok().map(|thumb| (child, thumb.0)))
		else {
			continue;
		};
		let Ok((scroll, viewport)) = viewports.get(thumb.viewport) else {
			continue;
		};
		let viewport_h = viewport.size().y * viewport.inverse_scale_factor();
		let content_h = viewport.content_size().y * viewport.inverse_scale_factor();
		let track_h = track_computed.size().y * track_computed.inverse_scale_factor();
		if content_h <= viewport_h + 1.0 || track_h <= 0.0 {
			*track_visibility = Visibility::Hidden;
			continue;
		}
		*track_visibility = Visibility::Inherited;
		let ratio = (viewport_h / content_h).clamp(0.08, 1.0);
		let thumb_h = (track_h * ratio).max(MIN_THUMB_PX).min(track_h);
		let max_scroll = (content_h - viewport_h).max(0.0);
		let max_top = (track_h - thumb_h).max(0.0);
		let top = if max_scroll <= 0.0 { 0.0 } else { (scroll.y / max_scroll) * max_top };
		if let Ok((_, mut thumb_node)) = thumbs.get_mut(thumb_entity) {
			thumb_node.height = Val::Px(thumb_h);
			thumb_node.top = Val::Px(top);
		}
	}
}

pub fn send_hud_scroll_events(
	stack: Res<MenuModalStack>,
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	if stack.owns_nav() {
		for _ in mouse_wheel_reader.read() {}
		return;
	}
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(HudScroll { entity, delta });
			}
		}
	}
}

pub fn on_hud_scroll(
	mut scroll: On<HudScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<HudScrollViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
		return;
	};
	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let delta = &mut scroll.delta;
	if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
		let max =
			if delta.x > 0. { scroll_position.x >= max_offset.x } else { scroll_position.x <= 0. };
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}
	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max =
			if delta.y > 0. { scroll_position.y >= max_offset.y } else { scroll_position.y <= 0. };
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}
	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}

/// Read-only panes (Loadout) have a [`HudMenu`] with no items, so arrows never
/// change selection. Step the viewport instead.
pub fn scroll_hud_viewport_on_nav(
	impulse: On<MenuNavImpulse>,
	mut viewports: Query<(&HudMenu, &ComputedNode, &mut ScrollPosition), With<HudScrollViewport>>,
) {
	let Ok((menu, computed, mut scroll)) = viewports.get_mut(impulse.entity) else {
		return;
	};
	if menu.item_count > 0 {
		return;
	}
	let Some(delta) = hud_nav_scroll_delta(impulse.event().nav) else {
		return;
	};
	add_scroll_y(&mut scroll, computed, delta);
}

fn hud_nav_scroll_delta(nav: MenuNav) -> Option<f32> {
	match nav {
		MenuNav::Down => Some(HUD_NAV_SCROLL_LINE),
		MenuNav::Up => Some(-HUD_NAV_SCROLL_LINE),
		_ => None,
	}
}

/// Keep the focused HUD item inside a scroll viewport (sliders, clothing / weapons).
pub fn scroll_hud_selection_into_view(
	mut viewports: Query<
		(Entity, &HudMenu, &ComputedNode, &mut ScrollPosition),
		With<HudScrollViewport>,
	>,
	items: Query<(Entity, &HudMenuItem, &ComputedNode, &bevy::ui::UiGlobalTransform)>,
	transforms: Query<&bevy::ui::UiGlobalTransform>,
	child_of: Query<&ChildOf>,
) {
	for (viewport, menu, computed, mut scroll) in &mut viewports {
		if menu.item_count == 0 {
			continue;
		}
		let Ok(view_tf) = transforms.get(viewport) else {
			continue;
		};
		let Some((_, _, item_node, item_tf)) = items.iter().find(|(entity, item, _, _)| {
			item.menu == viewport
				&& item.index == menu.selected
				&& entity_is_under(*entity, viewport, &child_of)
		}) else {
			continue;
		};
		reveal_item_in_viewport(computed, view_tf, item_node, item_tf, TILE_FOCUS_PAD, &mut scroll);
	}
}

pub(crate) fn reveal_item_in_viewport(
	view: &ComputedNode,
	view_tf: &bevy::ui::UiGlobalTransform,
	item: &ComputedNode,
	item_tf: &bevy::ui::UiGlobalTransform,
	slack_px: f32,
	scroll: &mut ScrollPosition,
) {
	let scale = view.inverse_scale_factor();
	let view_h = view.size().y;
	let item_h = item.size().y;
	if view_h <= 0.0 || item_h <= 0.0 {
		return;
	}
	let slack = slack_px / scale.max(f32::EPSILON);
	let delta = scroll_delta_to_reveal(
		view_tf.affine().translation.y,
		view_h,
		item_tf.affine().translation.y,
		item_h,
		slack,
	);
	if delta == 0.0 {
		return;
	}
	add_scroll_y(scroll, view, delta * scale);
}

fn add_scroll_y(scroll: &mut ScrollPosition, computed: &ComputedNode, delta: f32) {
	let scale = computed.inverse_scale_factor();
	let max_scroll = ((computed.content_size().y - computed.size().y) * scale).max(0.0);
	if max_scroll <= 0.0 {
		return;
	}
	scroll.y = (scroll.y + delta).clamp(0.0, max_scroll);
}

/// [`UiGlobalTransform`] is the node center. Delta is in the same space as the
/// sizes: negative scrolls up, positive scrolls down.
fn scroll_delta_to_reveal(
	view_center: f32,
	view_h: f32,
	item_center: f32,
	item_h: f32,
	slack: f32,
) -> f32 {
	let view_top = view_center - view_h * 0.5;
	let item_top = item_center - item_h * 0.5;
	if item_top < view_top + slack {
		item_top - (view_top + slack)
	} else if item_top + item_h > view_top + view_h - slack {
		item_top + item_h - (view_top + view_h - slack)
	} else {
		0.0
	}
}

pub(crate) fn entity_is_under(
	mut entity: Entity,
	root: Entity,
	child_of: &Query<&ChildOf>,
) -> bool {
	if entity == root {
		return true;
	}
	loop {
		let Ok(parent) = child_of.get(entity) else {
			return false;
		};
		if parent.parent() == root {
			return true;
		}
		entity = parent.parent();
	}
}

#[cfg(test)]
mod tests {
	use super::{hud_nav_scroll_delta, scroll_delta_to_reveal, HUD_NAV_SCROLL_LINE};
	use maybraid_input::MenuNav;

	#[test]
	fn item_already_visible_does_not_scroll() {
		assert_eq!(scroll_delta_to_reveal(400.0, 800.0, 400.0, 30.0, 6.0), 0.0);
	}

	#[test]
	fn item_below_the_fold_scrolls_down() {
		let delta = scroll_delta_to_reveal(400.0, 800.0, 900.0, 30.0, 6.0);
		assert!(delta > 0.0, "expected down, got {delta}");
	}

	#[test]
	fn treating_center_as_top_would_miss_the_below_item() {
		let view_center = 400.0;
		let view_h = 800.0;
		let item_center = 900.0;
		let item_h = 30.0;
		let slack = 6.0;
		let bogus_still_inside = item_center + item_h <= view_center + view_h - slack;
		assert!(bogus_still_inside);
		assert!(scroll_delta_to_reveal(view_center, view_h, item_center, item_h, slack) > 0.0);
	}

	#[test]
	fn item_above_the_fold_scrolls_up() {
		let delta = scroll_delta_to_reveal(400.0, 800.0, 20.0, 30.0, 6.0);
		assert!(delta < 0.0, "expected up, got {delta}");
	}

	#[test]
	fn empty_pane_maps_pad_arrows_to_scroll() {
		assert_eq!(hud_nav_scroll_delta(MenuNav::Down), Some(HUD_NAV_SCROLL_LINE));
		assert_eq!(hud_nav_scroll_delta(MenuNav::Up), Some(-HUD_NAV_SCROLL_LINE));
		assert_eq!(hud_nav_scroll_delta(MenuNav::Left), None);
		assert_eq!(hud_nav_scroll_delta(MenuNav::Select), None);
	}

	#[test]
	fn wheel_does_not_scroll_while_a_confirm_modal_owns_nav() {
		use super::{send_hud_scroll_events, HudScroll};
		use crate::modal::{MenuModalId, MenuModalLayer, MenuModalStack};
		use bevy::ecs::entity::EntityHashMap;
		use bevy::ecs::system::RunSystemOnce;
		use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
		use bevy::input::touch::TouchPhase;
		use bevy::picking::backend::HitData;
		use bevy::picking::hover::HoverMap;
		use bevy::picking::pointer::PointerId;
		use bevy::prelude::*;

		#[derive(Resource, Default)]
		struct ScrollHits(u32);

		fn count(scroll: On<HudScroll>, mut hits: ResMut<ScrollHits>) {
			hits.0 += 1;
			let _ = scroll;
		}

		fn wheel() -> MouseWheel {
			MouseWheel {
				unit: MouseScrollUnit::Line,
				x: 0.0,
				y: 1.0,
				window: Entity::PLACEHOLDER,
				phase: TouchPhase::Moved,
			}
		}

		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<HoverMap>();
		world.init_resource::<Messages<MouseWheel>>();
		world.init_resource::<ScrollHits>();
		world.add_observer(count);
		let viewport = world.spawn_empty().id();
		let mut hits = EntityHashMap::default();
		hits.insert(viewport, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
		world.resource_mut::<HoverMap>().0.insert(PointerId::Mouse, hits);
		world.resource_mut::<Messages<MouseWheel>>().write(wheel());
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		world.run_system_once(send_hud_scroll_events).expect("blocked wheel");
		world.flush();
		assert_eq!(world.resource::<ScrollHits>().0, 0);
		world.resource_mut::<MenuModalStack>().close(MenuModalId::ITEM_DROP);
		world.resource_mut::<ScrollHits>().0 = 0;
		world.resource_mut::<Messages<MouseWheel>>().write(wheel());
		world.run_system_once(send_hud_scroll_events).expect("restored wheel");
		world.flush();
		assert!(world.resource::<ScrollHits>().0 >= 1);
	}
}
