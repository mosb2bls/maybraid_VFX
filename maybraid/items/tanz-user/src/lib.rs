//! Tanzer control mode: left-stick / A / B gestures become equipped Tanzes
//! and always advance the Skill Map.

mod actions;
mod execute;
mod hud;
mod input;
mod stack;

use bevy::prelude::*;
use characters::CharacterMotionSystems;
use damage::Downed;
use maybraid_character_controller::{CharacterControlSystems, OrdinaryLocomotionEnabled};
use maybraid_input::VirtualPad;
use maybraid_skill_map::{SkillMapFlick, SkillMapSteerPolicy, SkillMapSystems};
use player::{MoveWish, PlayerLook, PlayerSystems};

pub use actions::{
	forward_dash, healing_salute, left_strafe, right_strafe, starter_stack, TanzAction,
};
pub use execute::{
	advance_tanz_execution, apply_tanz_movement, drive_tanz_animation, match_tanz_inputs,
	ActiveTanz, BufferedTanz, TanzExecution, TanzPhase,
};
pub use input::{
	CommittedGesture, ModeToggle, TanzButton, TanzInput, TanzRecognizer, TanzRecognizerConfig,
	TANZ_B_HOLD_SECS, TANZ_TOGGLE_SECS,
};
pub use stack::{TanzDefinition, TanzId, TanzInputRange, TanzStack, TanzTiming};

/// Standard locomotion versus Tanzer gesture play.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlMode {
	#[default]
	Standard,
	Tanzer,
}

impl ControlMode {
	pub fn toggle(self) -> Self {
		match self {
			Self::Standard => Self::Tanzer,
			Self::Tanzer => Self::Standard,
		}
	}

	pub fn is_tanzer(self) -> bool {
		self == Self::Tanzer
	}

	pub fn script_id(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Tanzer => "tanzer",
		}
	}

	pub fn from_script_id(raw: &str) -> Option<Self> {
		match raw.trim().to_ascii_lowercase().as_str() {
			"standard" | "loco" | "locomotion" => Some(Self::Standard),
			"tanzer" | "tanz" => Some(Self::Tanzer),
			_ => None,
		}
	}
}

/// Per-character Tanz ownership. Mode, settings, and the temporal recognizer
/// live here so a mode change can reset stale gestures in one place.
#[derive(Component, Clone, Debug)]
pub struct TanzUser {
	pub mode: ControlMode,
	pub settings: TanzUserSettings,
	pub recognizer: TanzRecognizer,
	pub toggle: ModeToggle,
}

impl Default for TanzUser {
	fn default() -> Self {
		Self {
			mode: ControlMode::Standard,
			settings: TanzUserSettings::default(),
			recognizer: TanzRecognizer::default(),
			toggle: ModeToggle::default(),
		}
	}
}

impl TanzUser {
	pub fn reset_input(&mut self) {
		self.recognizer.reset();
		self.toggle.reset();
	}

	/// Hold both bumpers long enough to flip modes. Clears recognizer, execution,
	/// and leftover movement wishes so the next mode starts clean.
	pub fn apply_toggle(
		&mut self,
		dt: f32,
		chord: bool,
		execution: &mut TanzExecution,
		wish: &mut MoveWish,
	) -> bool {
		if !self.toggle.sample(dt, chord, self.settings.toggle_secs) {
			return false;
		}
		self.mode = self.mode.toggle();
		self.recognizer.reset();
		execution.clear();
		wish.0 = Vec3::ZERO;
		info!("tanz control mode -> {:?}", self.mode);
		true
	}

	/// Set the mode directly (script / agent). Same cleanup as a chord toggle.
	pub fn set_mode(
		&mut self,
		mode: ControlMode,
		execution: &mut TanzExecution,
		wish: &mut MoveWish,
	) -> bool {
		if self.mode == mode {
			return false;
		}
		self.mode = mode;
		self.recognizer.reset();
		self.toggle.reset();
		execution.clear();
		wish.0 = Vec3::ZERO;
		info!("tanz control mode -> {:?}", self.mode);
		true
	}
}

/// Tunable recognition and toggle thresholds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TanzUserSettings {
	pub recognizer: TanzRecognizerConfig,
	pub toggle_secs: f32,
}

impl Default for TanzUserSettings {
	fn default() -> Self {
		Self { recognizer: TanzRecognizerConfig::default(), toggle_secs: TANZ_TOGGLE_SECS }
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TanzUserSystems {
	/// LB+RB chord and locomotion/map policy. Runs before pad dispatch.
	Toggle,
	/// Temporal recognition → [`TanzInput`].
	Recognize,
	/// Committed inputs → [`SkillMapFlick`]. Independent of Tanz matching.
	PublishMap,
	/// Stack matching and executor submit.
	Match,
	/// Lifecycle, cooldowns, and movement wishes.
	Execute,
	/// Full-body clip override after ordinary locomotion selection.
	Animate,
}

/// Stamp the starter Tanz stack on a live character.
pub fn spawn_tanz_user(commands: &mut Commands, user: Entity) {
	commands.entity(user).insert((
		TanzUser::default(),
		TanzStack::starter(),
		TanzExecution::default(),
	));
}

pub struct TanzUserPlugin;

impl Plugin for TanzUserPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<TanzInput>()
			.init_resource::<OrdinaryLocomotionEnabled>()
			.init_resource::<SkillMapSteerPolicy>()
			.configure_sets(
				Update,
				(
					TanzUserSystems::Toggle.before(CharacterControlSystems),
					TanzUserSystems::Recognize
						.after(TanzUserSystems::Toggle)
						.before(SkillMapSystems::Steer),
					TanzUserSystems::PublishMap
						.after(TanzUserSystems::Recognize)
						.before(SkillMapSystems::Steer),
					TanzUserSystems::Match.after(TanzUserSystems::Recognize),
					TanzUserSystems::Execute
						.after(TanzUserSystems::Match)
						.after(PlayerSystems::Intent)
						.before(PlayerSystems::Body),
					TanzUserSystems::Animate
						.after(PlayerSystems::Locomotion)
						.before(CharacterMotionSystems::Anim),
				),
			)
			.add_systems(
				Update,
				(toggle_control_mode, sync_tanz_policies)
					.chain()
					.in_set(TanzUserSystems::Toggle),
			)
			.add_systems(Update, recognize_tanz_inputs.in_set(TanzUserSystems::Recognize))
			.add_systems(Update, publish_tanz_map_inputs.in_set(TanzUserSystems::PublishMap))
			.add_systems(Update, match_tanz_inputs.in_set(TanzUserSystems::Match))
			.add_systems(
				Update,
				(advance_tanz_execution, apply_tanz_movement, interrupt_downed_tanzes)
					.chain()
					.in_set(TanzUserSystems::Execute),
			)
			.add_systems(Update, drive_tanz_animation.in_set(TanzUserSystems::Animate))
			.add_systems(Startup, hud::spawn_tanz_mode_hud)
			.add_systems(Update, hud::sync_tanz_mode_hud.after(TanzUserSystems::Toggle));
	}
}

fn toggle_control_mode(
	time: Res<Time>,
	pad: Option<Res<VirtualPad>>,
	mut users: Query<(&mut TanzUser, &mut TanzExecution, &mut MoveWish)>,
) {
	let Some(pad) = pad else {
		return;
	};
	let dt = time.delta_secs();
	let chord = pad.bumper_chord_held();
	for (mut user, mut execution, mut wish) in &mut users {
		user.apply_toggle(dt, chord, &mut execution, &mut wish);
	}
}

fn sync_tanz_policies(
	users: Query<&TanzUser>,
	mut ordinary: ResMut<OrdinaryLocomotionEnabled>,
	mut policy: ResMut<SkillMapSteerPolicy>,
) {
	let tanzer = users.iter().any(|user| user.mode.is_tanzer());
	ordinary.0 = !tanzer;
	*policy = if tanzer { SkillMapSteerPolicy::Tanz } else { SkillMapSteerPolicy::Standard };
}

fn recognize_tanz_inputs(
	time: Res<Time>,
	pad: Option<Res<VirtualPad>>,
	mut users: Query<(Entity, &mut TanzUser)>,
	mut inputs: MessageWriter<TanzInput>,
) {
	let Some(pad) = pad else {
		return;
	};
	let dt = time.delta_secs();
	for (entity, mut user) in &mut users {
		if !user.mode.is_tanzer() {
			continue;
		}
		let config = user.settings.recognizer;
		let Some(gesture) = user.recognizer.sample(dt, pad.move_stick, &pad, &config) else {
			continue;
		};
		inputs.write(TanzInput {
			user: entity,
			direction: gesture.direction,
			magnitude: gesture.magnitude,
			coherence: gesture.coherence,
			button: gesture.button,
		});
	}
}

fn publish_tanz_map_inputs(
	users: Query<&TanzUser>,
	mut inputs: MessageReader<TanzInput>,
	mut flicks: MessageWriter<SkillMapFlick>,
) {
	for input in inputs.read() {
		let Ok(user) = users.get(input.user) else {
			continue;
		};
		if !user.mode.is_tanzer() {
			continue;
		}
		let flick = input.direction * input.magnitude;
		if flick == Vec2::ZERO {
			continue;
		}
		flicks.write(SkillMapFlick(flick));
	}
}

fn interrupt_downed_tanzes(
	mut users: Query<(&mut TanzUser, &mut TanzExecution, &mut MoveWish), Added<Downed>>,
) {
	for (mut user, mut execution, mut wish) in &mut users {
		user.recognizer.reset();
		execution.clear();
		wish.0 = Vec3::ZERO;
	}
}

/// Camera-relative XZ wish from a stick-space vector.
pub fn look_wish(yaw: f32, stick: Vec2) -> Vec3 {
	if stick == Vec2::ZERO {
		return Vec3::ZERO;
	}
	let yaw = Quat::from_axis_angle(Vec3::Y, yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	(right * stick.x + forward * stick.y).clamp_length_max(1.0)
}

pub fn look_yaw(look: Option<&PlayerLook>) -> f32 {
	look.map(|look| look.yaw).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_skill_map::SkillMapFlick;

	#[test]
	fn script_ids_roundtrip() {
		assert_eq!(ControlMode::from_script_id("tanzer"), Some(ControlMode::Tanzer));
		assert_eq!(ControlMode::from_script_id("STANDARD"), Some(ControlMode::Standard));
		assert_eq!(ControlMode::Tanzer.script_id(), "tanzer");
		assert!(ControlMode::from_script_id("toggle").is_none());
	}

	#[test]
	fn set_mode_is_idempotent_and_clears() -> anyhow::Result<()> {
		let mut user = TanzUser { mode: ControlMode::Standard, ..TanzUser::default() };
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::ForwardDash, Vec2::Y, 0.5);
		let mut wish = MoveWish(Vec3::Y);
		assert!(!user.set_mode(ControlMode::Standard, &mut execution, &mut wish));
		assert!(user.set_mode(ControlMode::Tanzer, &mut execution, &mut wish));
		assert_eq!(user.mode, ControlMode::Tanzer);
		assert!(execution.active.is_none());
		assert_eq!(wish.0, Vec3::ZERO);
		Ok(())
	}

	#[test]
	fn bumper_chord_toggles_once_per_hold() -> anyhow::Result<()> {
		let mut user = TanzUser::default();
		let mut execution = TanzExecution::default();
		let mut wish = MoveWish::default();
		assert!(!user.apply_toggle(0.16, true, &mut execution, &mut wish));
		assert_eq!(user.mode, ControlMode::Standard);
		assert!(user.apply_toggle(0.16, true, &mut execution, &mut wish));
		assert_eq!(user.mode, ControlMode::Tanzer);
		assert!(!user.apply_toggle(0.16, true, &mut execution, &mut wish));
		assert!(!user.apply_toggle(0.16, false, &mut execution, &mut wish));
		assert!(user.apply_toggle(0.32, true, &mut execution, &mut wish));
		assert_eq!(user.mode, ControlMode::Standard);
		Ok(())
	}

	#[test]
	fn mode_change_clears_execution_and_wishes() -> anyhow::Result<()> {
		let mut user = TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() };
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::LeftStrafe, Vec2::NEG_X, 0.0);
		let mut wish = MoveWish(Vec3::X);
		assert!(user.apply_toggle(0.32, true, &mut execution, &mut wish));
		assert_eq!(user.mode, ControlMode::Standard);
		assert!(execution.active.is_none());
		assert_eq!(wish.0, Vec3::ZERO);
		Ok(())
	}

	#[test]
	fn tanzer_policy_disables_ordinary_locomotion() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<OrdinaryLocomotionEnabled>()
			.init_resource::<SkillMapSteerPolicy>()
			.add_systems(Update, sync_tanz_policies);
		app.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() });
		app.update();
		assert_eq!(*app.world().resource::<SkillMapSteerPolicy>(), SkillMapSteerPolicy::Tanz);
		assert!(!app.world().resource::<OrdinaryLocomotionEnabled>().0);
		Ok(())
	}

	#[test]
	fn unmatched_directional_input_still_advances_the_map() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, publish_tanz_map_inputs);
		let user = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.world_mut().write_message(TanzInput {
			user,
			direction: Vec2::NEG_X,
			magnitude: 0.9,
			coherence: 1.0,
			button: TanzButton::None,
		});
		app.update();
		let flicks: Vec<Vec2> = app
			.world_mut()
			.resource_mut::<Messages<SkillMapFlick>>()
			.drain()
			.map(|flick| flick.0)
			.collect();
		assert_eq!(flicks, vec![Vec2::NEG_X * 0.9]);
		Ok(())
	}

	#[test]
	fn neutral_button_only_input_does_not_invent_a_map_direction() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_message::<SkillMapFlick>()
			.add_systems(Update, publish_tanz_map_inputs);
		let user = app
			.world_mut()
			.spawn(TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() })
			.id();
		app.world_mut().write_message(TanzInput {
			user,
			direction: Vec2::ZERO,
			magnitude: 0.0,
			coherence: 0.0,
			button: TanzButton::APress,
		});
		app.update();
		let flicks: Vec<Vec2> = app
			.world_mut()
			.resource_mut::<Messages<SkillMapFlick>>()
			.drain()
			.map(|flick| flick.0)
			.collect();
		assert!(flicks.is_empty());
		Ok(())
	}
}
