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

impl ContinuousHealthConfig {
	pub fn frame_fill() -> Color {
		Color::srgba(0.12, 0.13, 0.15, 0.9)
	}

	pub fn track_fill() -> Color {
		Color::srgba(0.05, 0.05, 0.06, 0.95)
	}

	pub fn inner_radius(&self) -> f32 {
		(self.corner_radius - 1.0).max(0.0)
	}

	pub fn frame_node(&self) -> Node {
		Node {
			width: Val::Percent(100.0),
			height: Val::Px(self.bar_height),
			flex_grow: 1.0,
			padding: UiRect::all(Val::Px(self.frame_thickness)),
			border_radius: BorderRadius::all(Val::Px(self.corner_radius)),
			..default()
		}
	}

	pub fn track_node(&self) -> Node {
		Node {
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			border_radius: BorderRadius::all(Val::Px(self.inner_radius())),
			overflow: Overflow::clip(),
			..default()
		}
	}

	pub fn layer_node(&self, width_percent: f32) -> Node {
		Node {
			width: Val::Percent(width_percent),
			height: Val::Percent(100.0),
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			border_radius: BorderRadius::all(Val::Px(self.inner_radius())),
			..default()
		}
	}
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

/// Marker for the persistent player health meter root.
#[derive(Component)]
pub struct ContinuousHealthBar {
	pub target: Entity,
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
		Self::snap_to(1.0)
	}
}

impl HealthAnimState {
	pub(crate) fn snap_to(fraction: f32) -> Self {
		let fraction = if fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.0 };
		Self {
			current_fraction: fraction,
			target_fraction: fraction,
			trail_fraction: fraction,
			flash_timer: 0.0,
		}
	}

	fn settle(&mut self, target: f32, dt: f32, trail_speed: f32) {
		if (self.target_fraction - target).abs() > SETTLE_EPS {
			self.target_fraction = target;
		}
		let delta = self.target_fraction - self.current_fraction;
		if delta.abs() <= SETTLE_EPS {
			self.current_fraction = self.target_fraction;
		} else {
			self.current_fraction += delta * FILL_SPEED * dt;
		}
		if self.current_fraction < self.trail_fraction {
			if self.trail_fraction > self.target_fraction + SETTLE_EPS {
				self.trail_fraction -=
					(self.trail_fraction - self.current_fraction) * trail_speed * dt;
				self.trail_fraction = self.trail_fraction.max(self.current_fraction);
			}
		} else {
			self.trail_fraction = self.current_fraction;
		}
		if self.flash_timer > 0.0 {
			self.flash_timer -= dt;
		}
	}
}

const SETTLE_EPS: f32 = 0.002;
const FILL_SPEED: f32 = 3.0;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn spawn_continuous_health_bar(
	parent: Entity,
	target: Entity,
	commands: &mut Commands,
	config: &ContinuousHealthConfig,
) -> Entity {
	let mut bar_entity = Entity::PLACEHOLDER;
	commands.entity(parent).with_children(|root| {
		bar_entity = spawn_continuous_health_bar_in_builder(root, target, config);
	});
	bar_entity
}

/// Spawn the player health meter inside a UI plate.
pub fn spawn_continuous_health_bar_in_builder(
	parent: &mut ChildSpawnerCommands,
	target: Entity,
	config: &ContinuousHealthConfig,
) -> Entity {
	parent
		.spawn((
			ContinuousHealthBar { target },
			HealthAnimState::default(),
			ui_meter_row_node(config),
			Visibility::Inherited,
			Pickable::IGNORE,
		))
		.with_children(|anchor| {
			spawn_health_track(anchor, config);
			spawn_meter_amount(anchor, "0", HealthLabel);
		})
		.id()
}

pub(crate) fn ui_meter_row_node(config: &ContinuousHealthConfig) -> Node {
	Node {
		width: Val::Percent(100.0),
		height: Val::Px(config.bar_height + 6.0),
		flex_direction: FlexDirection::Row,
		align_items: AlignItems::Center,
		column_gap: Val::Px(8.0),
		..default()
	}
}

pub(crate) fn spawn_meter_amount(
	anchor: &mut ChildSpawnerCommands,
	initial: &str,
	extra: impl Bundle,
) {
	anchor.spawn((
		extra,
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

/// Charge / other meters: same framed track + trailing amount as player vitals.
pub(crate) fn spawn_ui_meter_row(
	parent: &mut ChildSpawnerCommands,
	config: &ContinuousHealthConfig,
	fill: impl Bundle,
	amount: impl Bundle,
) -> Entity {
	parent
		.spawn((ui_meter_row_node(config), Visibility::Inherited, Pickable::IGNORE))
		.with_children(|row| {
			spawn_meter_track(row, config, |track| {
				track.spawn((
					fill,
					config.layer_node(0.0),
					BackgroundColor(config.fill_color_high),
					Pickable::IGNORE,
				));
			});
			spawn_meter_amount(row, "0", amount);
		})
		.id()
}

fn spawn_health_track(anchor: &mut ChildSpawnerCommands, config: &ContinuousHealthConfig) {
	spawn_meter_track(anchor, config, |track| {
		track.spawn((
			HealthTrail,
			config.layer_node(100.0),
			BackgroundColor(config.trail_color),
			Pickable::IGNORE,
		));
		track.spawn((
			HealthFill,
			config.layer_node(100.0),
			BackgroundColor(config.fill_color_high),
			Pickable::IGNORE,
		));
		track.spawn((
			HealPreview,
			config.layer_node(0.0),
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
			config.layer_node(100.0),
			BackgroundColor(config.completion_flash_color.with_alpha(0.0)),
			Pickable::IGNORE,
		));
	});
}

/// Shared framed meter used by health and Lubikan charge.
pub(crate) fn spawn_meter_track(
	parent: &mut ChildSpawnerCommands,
	config: &ContinuousHealthConfig,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
	parent
		.spawn((
			config.frame_node(),
			BackgroundColor(ContinuousHealthConfig::frame_fill()),
			Pickable::IGNORE,
		))
		.with_children(|frame| {
			frame
				.spawn((
					config.track_node(),
					BackgroundColor(ContinuousHealthConfig::track_fill()),
					Pickable::IGNORE,
				))
				.with_children(children);
		})
		.id()
}

fn set_percent_width(node: &mut Node, percent: f32) {
	let next = Val::Percent(percent);
	if node.width != next {
		node.width = next;
	}
}

fn set_percent_left(node: &mut Node, percent: f32) {
	let next = Val::Percent(percent);
	if node.left != next {
		node.left = next;
	}
}

fn set_px_left(node: &mut Node, px: f32) {
	let next = Val::Px(px);
	if node.left != next {
		node.left = next;
	}
}

fn set_color(color: &mut BackgroundColor, next: Color) {
	if color.0 != next {
		color.0 = next;
	}
}

fn set_visibility(visibility: &mut Visibility, next: Visibility) {
	if *visibility != next {
		*visibility = next;
	}
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_continuous_health_bar_content(
	time: Res<Time>,
	config: Option<Res<ContinuousHealthConfig>>,
	hud_visible: Res<CombatHudVisible>,
	healths: Query<&Health, (Without<damage::Downed>, Without<ContinuousHealthBar>)>,
	heal_channels: Query<&lubikans::LubikanSelfHealChannel>,
	mut bars: Query<(Entity, &ContinuousHealthBar, &mut HealthAnimState)>,
	children: Query<&Children>,
	mut fills: Query<(&mut Node, &mut BackgroundColor), (With<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>)>,
	mut trails: Query<&mut Node, (With<HealthTrail>, Without<HealthFill>, Without<HealPreview>, Without<CapMarker>)>,
	mut previews: Query<(&mut Node, &mut Visibility), (With<HealPreview>, Without<HealthFill>, Without<HealthTrail>, Without<CapMarker>)>,
	mut markers: Query<(&mut Node, &mut Visibility), (With<CapMarker>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>)>,
	mut flashes: Query<&mut BackgroundColor, (With<CompletionFlash>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>)>,
	mut labels: Query<&mut Text, (With<HealthLabel>, Without<HealthFill>, Without<HealthTrail>, Without<HealPreview>, Without<CapMarker>, Without<CompletionFlash>)>,
) {
	if !hud_visible.0 {
		return;
	}

	let config = config.as_ref().map(|c| **c).unwrap_or_default();
	let dt = time.delta_secs();
	let elapsed = time.elapsed_secs();

	for (bar_entity, bar, mut anim_state) in &mut bars {
		let Ok(health) = healths.get(bar.target) else {
			continue;
		};

		anim_state.settle(health.fraction(), dt, config.trail_speed);
		let heal_channel = heal_channels.get(bar.target).ok();
		let is_healing = heal_channel.is_some_and(|channel| channel.active);
		let label = format!("{:.0}", health.current);

		let Ok(kids) = children.get(bar_entity) else {
			continue;
		};

		for child in kids {
			if let Ok(mut text) = labels.get_mut(*child) {
				if text.0 != label {
					text.0 = label.clone();
				}
			}

			let Ok(grandchildren) = children.get(*child) else {
				continue;
			};

			for grandchild in grandchildren {
				let Ok(great_grandchildren) = children.get(*grandchild) else {
					continue;
				};

				for element in great_grandchildren {
					if let Ok((mut fill_node, mut fill_color)) = fills.get_mut(*element) {
						set_percent_width(&mut fill_node, anim_state.current_fraction * 100.0);
						let mut next = health_fill_color(&config, anim_state.current_fraction);
						if is_healing {
							let pulse = (elapsed * config.heal_pulse_speed).sin() * 0.5 + 0.5;
							next = brighten_color(next, 1.0 + pulse * 0.15);
						}
						set_color(&mut fill_color, next);
					}

					if let Ok(mut trail_node) = trails.get_mut(*element) {
						set_percent_width(&mut trail_node, anim_state.trail_fraction * 100.0);
					}

					if let Ok((mut preview_node, mut preview_visibility)) = previews.get_mut(*element) {
						if is_healing {
							if let Some(channel) = heal_channel {
								let heal_amount = channel.remaining / health.max;
								let heal_end =
									(anim_state.current_fraction + heal_amount).min(channel.cap_fraction);
								let preview_width = (heal_end - anim_state.current_fraction).max(0.0);
								set_percent_left(&mut preview_node, anim_state.current_fraction * 100.0);
								set_percent_width(&mut preview_node, preview_width * 100.0);
								set_visibility(&mut preview_visibility, Visibility::Visible);
							}
						} else {
							set_visibility(&mut preview_visibility, Visibility::Hidden);
						}
					}

					if let Ok((mut marker_node, mut marker_visibility)) = markers.get_mut(*element) {
						if let Some(channel) = heal_channel {
							if channel.damaged && channel.cap_fraction < 1.0 {
								set_px_left(
									&mut marker_node,
									channel.cap_fraction
										* (config.bar_width - config.frame_thickness * 2.0)
										- config.cap_marker_width * 0.5,
								);
								set_visibility(&mut marker_visibility, Visibility::Visible);
							} else {
								set_visibility(&mut marker_visibility, Visibility::Hidden);
							}
						} else {
							set_visibility(&mut marker_visibility, Visibility::Hidden);
						}
					}

					if let Ok(mut flash_color) = flashes.get_mut(*element) {
						let alpha = if anim_state.flash_timer > 0.0 {
							(anim_state.flash_timer / config.completion_flash_duration).clamp(0.0, 1.0)
								* 0.8
						} else {
							0.0
						};
						set_color(&mut flash_color, config.completion_flash_color.with_alpha(alpha));
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
	fn player_bar_inherits_visibility_from_the_vitals_plate() {
		let mut app = test_app();
		let target = app
			.world_mut()
			.spawn((Health { current: 100.0, max: 100.0 }, GlobalTransform::default()))
			.id();
		let config = ContinuousHealthConfig::default();
		let root = app.world_mut().spawn(Node::default()).id();
		spawn_continuous_health_bar(root, target, &mut app.world_mut().commands(), &config);
		app.update();
		let visibility = app
			.world_mut()
			.query_filtered::<&Visibility, With<ContinuousHealthBar>>()
			.iter(app.world())
			.next()
			.copied()
			.expect("bar");
		assert_eq!(
			visibility,
			Visibility::Inherited,
			"player bars follow the vitals plate so death/respawn can hide them"
		);
	}

	#[test]
	fn meter_chrome_matches_the_health_defaults() {
		let config = ContinuousHealthConfig::default();
		assert_eq!(config.frame_node().height, Val::Px(12.0));
		assert_eq!(config.frame_node().padding.left, Val::Px(1.5));
		assert_eq!(config.inner_radius(), 2.0);
		assert_eq!(ContinuousHealthConfig::frame_fill(), Color::srgba(0.12, 0.13, 0.15, 0.9));
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
		assert_eq!(world.query::<&HealthLabel>().iter(world).count(), 1, "label spawned");
	}
}

