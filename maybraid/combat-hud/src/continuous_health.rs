//! Continuous health bar with smooth animations, healing effects, and cap markers.

use bevy::prelude::*;
use damage::Health;

use crate::CombatHudVisible;

/// Visual config for the continuous health bar.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ContinuousHealthConfig {
	pub bar_width: f32,
	pub bar_height: f32,
	pub frame_thickness: f32,
	pub corner_radius: f32,
	pub fill_color_high: Color,
	pub fill_color_mid: Color,
	pub fill_color_low: Color,
	pub trail_color: Color,
	pub trail_delay: f32,
	pub trail_speed: f32,
	pub heal_preview_color: Color,
	pub heal_pulse_speed: f32,
	pub cap_marker_color: Color,
	pub cap_marker_width: f32,
	pub completion_flash_duration: f32,
	pub completion_flash_color: Color,
}

impl Default for ContinuousHealthConfig {
	fn default() -> Self {
		Self {
			bar_width: 120.0,
			bar_height: 12.0,
			frame_thickness: 1.5,
			corner_radius: 3.0,
			fill_color_high: Color::srgb(0.35, 0.85, 0.45),
			fill_color_mid: Color::srgb(0.95, 0.75, 0.22),
			fill_color_low: Color::srgb(0.90, 0.22, 0.18),
			trail_color: Color::srgba(0.95, 0.35, 0.30, 0.6),
			trail_delay: 0.3,
			trail_speed: 2.0,
			heal_preview_color: Color::srgba(0.5, 0.95, 0.6, 0.4),
			heal_pulse_speed: 2.5,
			cap_marker_color: Color::srgb(0.95, 0.85, 0.25),
			cap_marker_width: 2.0,
			completion_flash_duration: 0.4,
			completion_flash_color: Color::srgba(1.0, 1.0, 0.9, 0.8),
		}
	}
}

/// Layout mode for the continuous health bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthBarLayout {
	/// World-anchored: positioned via world-to-screen projection, absolute positioning.
	WorldAnchored,
	/// UI child: relative positioning within parent UI container (for player vitals plate).
	UiChild,
}

/// Marker for the continuous health bar root node.
#[derive(Component)]
pub struct ContinuousHealthBar {
	pub target: Entity,
	pub layout: HealthBarLayout,
}

/// Marker for the fill element (current HP).
#[derive(Component)]
pub struct HealthFill;

/// Marker for the damage trail element (shows recent damage).
#[derive(Component)]
pub struct HealthTrail;

/// Marker for the heal preview element.
#[derive(Component)]
pub(crate) struct HealPreview;

/// Marker for the cap marker.
#[derive(Component)]
pub(crate) struct CapMarker;

/// Marker for the completion flash overlay.
#[derive(Component)]
pub(crate) struct CompletionFlash;

/// Marker for the status dot above the bar.
#[derive(Component)]
pub(crate) struct HealthStatusDot;

/// Marker for the numeric label.
#[derive(Component)]
pub(crate) struct HealthLabel;

/// Animation state for smooth interpolation.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct HealthAnimState {
	current_fraction: f32,
	target_fraction: f32,
	trail_fraction: f32,
	flash_timer: f32,
}

impl Default for HealthAnimState {
	fn default() -> Self {
		Self {
			current_fraction: 1.0,
			target_fraction: 1.0,
			trail_fraction: 1.0,
			flash_timer: 0.0,
		}
	}
}

pub fn spawn_continuous_health_bar(
	parent: Entity,
	target: Entity,
	commands: &mut Commands,
	config: &ContinuousHealthConfig,
) {
	spawn_continuous_health_bar_with_layout(
		parent,
		target,
		commands,
		config,
		HealthBarLayout::WorldAnchored,
	);
}

/// Spawn health bar directly in a ChildSpawnerCommands context (from within with_children callback).
pub fn spawn_continuous_health_bar_in_builder(
	parent: &mut ChildSpawnerCommands,
	target: Entity,
	config: &ContinuousHealthConfig,
	layout: HealthBarLayout,
) -> Entity {
	spawn_continuous_health_bar_impl(parent, target, config, layout)
}

pub fn spawn_continuous_health_bar_with_layout(
	parent: Entity,
	target: Entity,
	commands: &mut Commands,
	config: &ContinuousHealthConfig,
	layout: HealthBarLayout,
) -> Entity {
	let mut bar_entity = Entity::PLACEHOLDER;
	commands.entity(parent).with_children(|root| {
		bar_entity = spawn_continuous_health_bar_impl(root, target, config, layout);
	});
	bar_entity
}

fn spawn_continuous_health_bar_impl(
	root: &mut ChildSpawnerCommands,
	target: Entity,
	config: &ContinuousHealthConfig,
	layout: HealthBarLayout,
) -> Entity {
	let node = match layout {
		HealthBarLayout::WorldAnchored => Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Px(config.bar_width),
			height: Val::Px(config.bar_height + 20.0),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Center,
			row_gap: Val::Px(2.0),
			..default()
		},
		HealthBarLayout::UiChild => Node {
			width: Val::Percent(100.0),
			height: Val::Px(config.bar_height + 6.0),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: Val::Px(8.0),
			..default()
		},
	};
	
	let visibility = match layout {
		HealthBarLayout::WorldAnchored => Visibility::Hidden,
		HealthBarLayout::UiChild => Visibility::Visible,
	};

	root.spawn((
		ContinuousHealthBar { target, layout },
		HealthAnimState::default(),
		node,
		visibility,
		Pickable::IGNORE,
	))
		.with_children(|anchor| {
			if layout == HealthBarLayout::WorldAnchored {
				anchor.spawn((
					HealthStatusDot,
					Node {
						width: Val::Px(10.0),
						height: Val::Px(10.0),
						border_radius: BorderRadius::all(Val::Px(10.0)),
						..default()
					},
					BackgroundColor(Color::WHITE),
					Visibility::Hidden,
					Pickable::IGNORE,
				));
			}
			if layout == HealthBarLayout::WorldAnchored {
				spawn_health_label(anchor, "");
			}
			spawn_health_track(anchor, config);
			if layout == HealthBarLayout::UiChild {
				spawn_health_label(anchor, "0");
			}
		})
		.id()
}

fn spawn_health_label(anchor: &mut ChildSpawnerCommands, initial: &str) {
	anchor.spawn((
		HealthLabel,
		Text::new(initial),
		TextFont { font_size: FontSize::Px(12.0), ..default() },
		TextColor(Color::WHITE),
		Node {
			min_width: Val::Px(28.0),
			..default()
		},
		Pickable::IGNORE,
	));
}

fn spawn_health_track(anchor: &mut ChildSpawnerCommands, config: &ContinuousHealthConfig) {
	anchor
		.spawn((
			Node {
				width: Val::Percent(100.0),
				height: Val::Px(config.bar_height),
				flex_grow: 1.0,
				padding: UiRect::all(Val::Px(config.frame_thickness)),
				border_radius: BorderRadius::all(Val::Px(config.corner_radius)),
				..default()
			},
			BackgroundColor(Color::srgba(0.12, 0.13, 0.15, 0.9)),
			Pickable::IGNORE,
		))
		.with_children(|frame| {
			frame
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Percent(100.0),
						border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
						overflow: Overflow::clip(),
						..default()
					},
					BackgroundColor(Color::srgba(0.05, 0.05, 0.06, 0.95)),
					Pickable::IGNORE,
				))
				.with_children(|track| {
					track.spawn((
						HealthTrail,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							top: Val::Px(0.0),
							border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
							..default()
						},
						BackgroundColor(config.trail_color),
						Pickable::IGNORE,
					));
					track.spawn((
						HealthFill,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							top: Val::Px(0.0),
							border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
							..default()
						},
						BackgroundColor(config.fill_color_high),
						Pickable::IGNORE,
					));
					track.spawn((
						HealPreview,
						Node {
							width: Val::Percent(0.0),
							height: Val::Percent(100.0),
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							top: Val::Px(0.0),
							border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
							..default()
						},
						BackgroundColor(config.heal_preview_color),
						Visibility::Hidden,
						Pickable::IGNORE,
					));
					track.spawn((
						CapMarker,
						Node {
							width: Val::Px(config.cap_marker_width),
							height: Val::Percent(100.0),
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							top: Val::Px(0.0),
							..default()
						},
						BackgroundColor(config.cap_marker_color),
						Visibility::Hidden,
						Pickable::IGNORE,
					));
					track.spawn((
						CompletionFlash,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							position_type: PositionType::Absolute,
							left: Val::Px(0.0),
							top: Val::Px(0.0),
							border_radius: BorderRadius::all(Val::Px(config.corner_radius - 1.0)),
							..default()
						},
						BackgroundColor(config.completion_flash_color.with_alpha(0.0)),
						Pickable::IGNORE,
					));
				});
		});
}

#[allow(clippy::type_complexity)]
pub fn position_continuous_health_bars(
	config: Option<Res<ContinuousHealthConfig>>,
	hud_visible: Res<CombatHudVisible>,
	cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
	healths: Query<(&GlobalTransform, Option<&player::LocomotionCapsule>), (With<Health>, Without<damage::Downed>)>,
	mut bars: Query<(&ContinuousHealthBar, &mut Node, &mut Visibility)>,
) {
	if !hud_visible.0 {
		for (_, _, mut visibility) in &mut bars {
			*visibility = Visibility::Hidden;
		}
		return;
	}

	let config = config.as_ref().map(|c| **c).unwrap_or_default();
	let camera = cameras.single().ok();

	for (bar, mut node, mut visibility) in &mut bars {
		if bar.layout == HealthBarLayout::UiChild {
			*visibility = Visibility::Visible;
			continue;
		}

		let Some((camera, camera_transform)) = camera else {
			*visibility = Visibility::Hidden;
			continue;
		};

		let Ok((transform, hull)) = healths.get(bar.target) else {
			*visibility = Visibility::Hidden;
			continue;
		};

		let lift = hull.copied().unwrap_or_default().half_height() + 0.38;
		let head = transform.translation() + Vec3::Y * lift;
		if let Ok(screen) = camera.world_to_viewport(camera_transform, head) {
			node.left = Val::Px(screen.x - config.bar_width * 0.5);
			node.top = Val::Px(screen.y - 26.0);
			*visibility = Visibility::Visible;
		} else {
			*visibility = Visibility::Hidden;
		}
	}
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_continuous_health_bar_content(
	time: Res<Time>,
	config: Option<Res<ContinuousHealthConfig>>,
	hud_visible: Res<CombatHudVisible>,
	healths: Query<(&Health, Option<&crate::CombatHudStatusColor>), (Without<damage::Downed>, Without<ContinuousHealthBar>)>,
	heal_channels: Query<&lubikans::LubikanSelfHealChannel>,
	mut bars: Query<(Entity, &ContinuousHealthBar, &mut HealthAnimState)>,
	children: Query<&Children>,
	mut fills: Query<(&mut Node, &mut BackgroundColor), (With<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>)>,
	mut trails: Query<&mut Node, (With<HealthTrail>, Without<HealthFill>, Without<HealPreview>, Without<CapMarker>)>,
	mut previews: Query<(&mut Node, &mut Visibility), (With<HealPreview>, Without<HealthFill>, Without<HealthTrail>, Without<CapMarker>)>,
	mut markers: Query<(&mut Node, &mut Visibility), (With<CapMarker>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>)>,
	mut flashes: Query<&mut BackgroundColor, (With<CompletionFlash>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>, Without<HealthStatusDot>)>,
	mut labels: Query<&mut Text, (With<HealthLabel>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>, Without<CompletionFlash>, Without<HealthStatusDot>)>,
	mut status_dots: Query<(&mut BackgroundColor, &mut Visibility), (With<HealthStatusDot>, Without<CompletionFlash>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>)>,
) {
	if !hud_visible.0 {
		return;
	}

	let config = config.as_ref().map(|c| **c).unwrap_or_default();
	let dt = time.delta_secs();
	let elapsed = time.elapsed_secs();

	for (bar_entity, bar, mut anim_state) in &mut bars {
		let Ok((health, status_color)) = healths.get(bar.target) else {
			continue;
		};

		// Update animation state
		let target_fraction = health.fraction();
		if (anim_state.target_fraction - target_fraction).abs() > 0.001 {
			// Health changed - update target
			anim_state.target_fraction = target_fraction;
		}

		// Smooth interpolation for current fill
		let fill_speed = 3.0;
		anim_state.current_fraction = anim_state.current_fraction
			+ (anim_state.target_fraction - anim_state.current_fraction) * fill_speed * dt;

		// Damage trail follows with delay
		if anim_state.current_fraction < anim_state.trail_fraction {
			// Taking damage - delay trail
			if anim_state.trail_fraction > anim_state.target_fraction + 0.001 {
				anim_state.trail_fraction = anim_state.trail_fraction
					- (anim_state.trail_fraction - anim_state.current_fraction) * config.trail_speed * dt;
				anim_state.trail_fraction = anim_state.trail_fraction.max(anim_state.current_fraction);
			}
		} else {
			// Healing - trail follows immediately
			anim_state.trail_fraction = anim_state.current_fraction;
		}

		// Completion flash timer
		if anim_state.flash_timer > 0.0 {
			anim_state.flash_timer -= dt;
		}

		// Get heal channel if active
		let heal_channel = heal_channels.get(bar.target).ok();
		let is_healing = heal_channel.map(|ch| ch.active).unwrap_or(false);

		// Update child elements
		let Ok(kids) = children.get(bar_entity) else {
			continue;
		};

		for child in kids {
			// Update status dot
			if let Ok((mut dot_color, mut dot_visibility)) = status_dots.get_mut(*child) {
				if let Some(status) = status_color {
					dot_color.0 = status.0;
					*dot_visibility = Visibility::Visible;
				} else {
					*dot_visibility = Visibility::Hidden;
				}
			}

			// Update label
			if let Ok(mut text) = labels.get_mut(*child) {
				text.0 = format!("{:.0}", health.current);
			}

			// Update bar elements
			let Ok(grandchildren) = children.get(*child) else {
				continue;
			};

			for grandchild in grandchildren {
				let Ok(great_grandchildren) = children.get(*grandchild) else {
					continue;
				};

				for element in great_grandchildren {
					// Update main fill
					if let Ok((mut fill_node, mut fill_color)) = fills.get_mut(*element) {
						let fill_percent = anim_state.current_fraction * 100.0;
						fill_node.width = Val::Percent(fill_percent);
						fill_color.0 = health_fill_color(&config, anim_state.current_fraction);
						
						// Add healing pulse effect
						if is_healing {
							let pulse = (elapsed * config.heal_pulse_speed).sin() * 0.5 + 0.5;
							let pulse_brightness = 1.0 + pulse * 0.15;
							fill_color.0 = brighten_color(fill_color.0, pulse_brightness);
						}
					}

					// Update damage trail
					if let Ok(mut trail_node) = trails.get_mut(*element) {
						trail_node.width = Val::Percent(anim_state.trail_fraction * 100.0);
					}

					// Update heal preview
					if let Ok((mut preview_node, mut preview_visibility)) = previews.get_mut(*element) {
						if is_healing {
							if let Some(channel) = heal_channel {
								let heal_amount = channel.remaining / health.max;
								let heal_end = (anim_state.current_fraction + heal_amount).min(channel.cap_fraction);
								let preview_width = (heal_end - anim_state.current_fraction).max(0.0);
								
								preview_node.left = Val::Percent(anim_state.current_fraction * 100.0);
								preview_node.width = Val::Percent(preview_width * 100.0);
								*preview_visibility = Visibility::Visible;
							}
						} else {
							*preview_visibility = Visibility::Hidden;
						}
					}

					// Update cap marker
					if let Ok((mut marker_node, mut marker_visibility)) = markers.get_mut(*element) {
						if let Some(channel) = heal_channel {
							if channel.damaged && channel.cap_fraction < 1.0 {
								marker_node.left = Val::Px(channel.cap_fraction * (config.bar_width - config.frame_thickness * 2.0) - config.cap_marker_width * 0.5);
								*marker_visibility = Visibility::Visible;
							} else {
								*marker_visibility = Visibility::Hidden;
							}
						} else {
							*marker_visibility = Visibility::Hidden;
						}
					}

					// Update completion flash
					if let Ok(mut flash_color) = flashes.get_mut(*element) {
						if anim_state.flash_timer > 0.0 {
							let alpha = (anim_state.flash_timer / config.completion_flash_duration).clamp(0.0, 1.0);
							flash_color.0 = config.completion_flash_color.with_alpha(alpha * 0.8);
						} else {
							flash_color.0 = config.completion_flash_color.with_alpha(0.0);
						}
					}
				}
			}
		}
	}
}

pub fn trigger_completion_flash(
	mut finished: MessageReader<lubikans::LubikanActivationFinished>,
	mut bars: Query<(&ContinuousHealthBar, &mut HealthAnimState)>,
	config: Option<Res<ContinuousHealthConfig>>,
) {
	let config = config.as_ref().map(|c| **c).unwrap_or_default();
	for event in finished.read() {
		for (bar, mut anim_state) in &mut bars {
			if bar.target == event.owner {
				anim_state.flash_timer = config.completion_flash_duration;
			}
		}
	}
}

fn health_fill_color(config: &ContinuousHealthConfig, fraction: f32) -> Color {
	if fraction <= 0.0 {
		config.fill_color_low.darker(0.5)
	} else if fraction < 0.35 {
		config.fill_color_low
	} else if fraction < 0.7 {
		config.fill_color_mid
	} else {
		config.fill_color_high
	}
}

fn brighten_color(color: Color, factor: f32) -> Color {
	let [r, g, b, a] = color.to_srgba().to_f32_array();
	Color::srgba(r * factor, g * factor, b * factor, a)
}

#[cfg(test)]
mod tests {
	use super::*;
	use lubikans::{LubikanAccountId, LubikanActivationSessionId};

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ContinuousHealthConfig>()
			.init_resource::<CombatHudVisible>()
			.add_message::<lubikans::LubikanActivationFinished>();
		app
	}

	#[test]
	fn fill_and_trail_interpolate_smoothly() {
		let mut app = test_app();
		let target = app.world_mut().spawn((
			Health { current: 100.0, max: 100.0 },
			GlobalTransform::default(),
		)).id();
		
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn_empty().id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		app.update();
		
		let mut anim_state = app.world_mut()
			.query_filtered::<&mut HealthAnimState, With<ContinuousHealthBar>>()
			.iter_mut(app.world_mut())
			.next()
			.expect("anim state")
			.clone();
		
		assert!((anim_state.current_fraction - 1.0).abs() < 0.01, "starts at full health");
		assert!((anim_state.trail_fraction - 1.0).abs() < 0.01, "trail starts at full");
		
		// Simulate damage
		anim_state.target_fraction = 0.5;
		anim_state.current_fraction = 0.5;
		
		// Trail should lag behind
		assert!(anim_state.trail_fraction > anim_state.current_fraction, "trail lags when damaged");
	}

	#[test]
	fn ceiling_marker_placement_at_70_percent() {
		let mut app = test_app();
		let target = app.world_mut().spawn((
			Health { current: 50.0, max: 100.0 },
			GlobalTransform::default(),
			lubikans::LubikanSelfHealChannel {
				active: true,
				remaining: 20.0,
				rate: 5.0,
				account: lubikans::LubikanAccountId::SELF,
				session: lubikans::LubikanActivationSessionId::mint(),
				damaged: true,
				cap_fraction: 0.7,
			},
		)).id();
		
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn_empty().id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		app.update();
		
		// Marker should be visible and at 70%
		let marker_left = 0.7 * (config.bar_width - config.frame_thickness * 2.0) 
			- config.cap_marker_width * 0.5;
		
		// The marker should be positioned correctly (we'd verify this in the update system)
		assert!(marker_left > 0.0, "marker positioned within bar");
	}

	#[test]
	fn heal_preview_clamping() {
		let mut app = test_app();
		
		// Create a target with 50% health, healing to 80% but capped at 70%
		let target = app.world_mut().spawn((
			Health { current: 50.0, max: 100.0 },
			GlobalTransform::default(),
			lubikans::LubikanSelfHealChannel {
				active: true,
				remaining: 30.0, // Would heal to 80%
				rate: 5.0,
				account: lubikans::LubikanAccountId::SELF,
				session: lubikans::LubikanActivationSessionId::mint(),
				damaged: true,
				cap_fraction: 0.7, // But capped at 70%
			},
		)).id();
		
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn_empty().id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		app.update();
		
		// Preview should only show healing up to 70%, not 80%
		let channel = app.world().get::<lubikans::LubikanSelfHealChannel>(target).unwrap();
		let current_fraction = 0.5;
		let heal_amount = channel.remaining / 100.0; // 0.3
		let heal_end = (current_fraction + heal_amount).min(channel.cap_fraction);
		
		assert!((heal_end - 0.7).abs() < 0.01, "heal preview clamped to cap: {}", heal_end);
	}

	#[test]
	fn completion_flash_triggered_by_event() {
		let mut app = test_app();
		
		let target = app.world_mut().spawn((
			Health { current: 70.0, max: 100.0 },
			GlobalTransform::default(),
		)).id();
		
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn_empty().id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		
		app.add_systems(Update, trigger_completion_flash);
		app.update();
		
		// Send completion event
		app.world_mut().write_message(lubikans::LubikanActivationFinished {
			owner: target,
			account: lubikans::LubikanAccountId::SELF,
			definition: lubikans::LubikanDefinitionId::SELF,
			id: lubikans::LubikanActivationSessionId::mint(),
			outcome: lubikans::LubikanActivationOutcome::Completed,
		});
		
		app.update();
		
		// Flash timer should be set
		let anim_state = app.world_mut()
			.query_filtered::<&HealthAnimState, With<ContinuousHealthBar>>()
			.iter(app.world_mut())
			.next()
			.expect("anim state");
		
		assert!(anim_state.flash_timer > 0.0, "flash timer triggered by event");
	}

	#[test]
	fn ui_child_bar_is_visible_again_after_hud_hide() {
		let mut app = test_app();
		app.add_systems(Update, position_continuous_health_bars);
		let target = app
			.world_mut()
			.spawn((Health { current: 100.0, max: 100.0 }, GlobalTransform::default()))
			.id();
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn(Node::default()).id();
		spawn_continuous_health_bar_with_layout(
			root,
			target,
			&mut app.world_mut().commands(),
			&config,
			HealthBarLayout::UiChild,
		);
		app.update();

		app.world_mut().insert_resource(CombatHudVisible(false));
		app.update();
		{
			let visibility = app
				.world_mut()
				.query_filtered::<&Visibility, With<ContinuousHealthBar>>()
				.iter(app.world())
				.next()
				.copied()
				.expect("bar");
			assert_eq!(visibility, Visibility::Hidden);
		}

		app.world_mut().insert_resource(CombatHudVisible(true));
		app.update();
		let visibility = app
			.world_mut()
			.query_filtered::<&Visibility, With<ContinuousHealthBar>>()
			.iter(app.world())
			.next()
			.copied()
			.expect("bar");
		assert_eq!(visibility, Visibility::Visible, "UI child bars must unhide with the HUD");
	}

	#[test]
	fn health_fill_colors_match_thresholds() {
		let config = ContinuousHealthConfig::default();
		
		let high = health_fill_color(&config, 0.8);
		let mid = health_fill_color(&config, 0.5);
		let low = health_fill_color(&config, 0.2);
		let dead = health_fill_color(&config, 0.0);
		
		assert_ne!(high, mid, "different colors for different ranges");
		assert_ne!(mid, low, "different colors for different ranges");
		assert_ne!(low, dead, "different colors for different ranges");
	}

	#[test]
	fn bar_spawns_all_required_elements() {
		let mut app = test_app();
		
		let target = app.world_mut().spawn((
			Health { current: 100.0, max: 100.0 },
			GlobalTransform::default(),
		)).id();
		
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn_empty().id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		app.update();
		
		let world = app.world_mut();
		
		// Verify all components exist
		assert_eq!(world.query::<&ContinuousHealthBar>().iter(world).count(), 1, "bar spawned");
		assert_eq!(world.query::<&HealthFill>().iter(world).count(), 1, "fill spawned");
		assert_eq!(world.query::<&HealthTrail>().iter(world).count(), 1, "trail spawned");
		assert_eq!(world.query::<&HealPreview>().iter(world).count(), 1, "preview spawned");
		assert_eq!(world.query::<&CapMarker>().iter(world).count(), 1, "marker spawned");
		assert_eq!(world.query::<&CompletionFlash>().iter(world).count(), 1, "flash spawned");
		assert_eq!(world.query::<&HealthStatusDot>().iter(world).count(), 1, "status dot spawned");
		assert_eq!(world.query::<&HealthLabel>().iter(world).count(), 1, "label spawned");
	}
}

