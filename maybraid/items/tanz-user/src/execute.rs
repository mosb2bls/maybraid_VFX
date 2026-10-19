//! Tanz lifecycle, cooldowns, chaining, movement, and clip override.

use std::collections::HashMap;

use bevy::prelude::*;
use characters::{
	AnimProgress, AnimRef, AnimRefRoot, CharacterMembers, CharacterRig, CharacterRigRole,
	CharacterRoot,
};
use damage::Health;
use player::{MoveSpeedCap, MoveWish, PlayerLook};

use crate::actions::{dash_ground_speed, TanzAction};
use crate::input::TanzInput;
use crate::stack::{TanzId, TanzStack};
use crate::{look_wish, look_yaw, ControlMode, TanzDefinitionExt, TanzUser};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TanzPhase {
	#[default]
	Startup,
	Active,
	Recovery,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveTanz {
	pub id: TanzId,
	pub age: f32,
	pub phase: TanzPhase,
	pub applied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BufferedTanz {
	pub id: TanzId,
	pub age: f32,
}

#[derive(Component, Clone, Debug, Default)]
pub struct TanzExecution {
	pub active: Option<ActiveTanz>,
	pub buffered: Option<BufferedTanz>,
	pub cooldowns: HashMap<TanzId, f32>,
}

impl TanzExecution {
	pub fn clear(&mut self) {
		self.active = None;
		self.buffered = None;
		self.cooldowns.clear();
	}

	pub fn cooling(&self, id: TanzId) -> bool {
		self.cooldowns.get(&id).is_some_and(|remaining| *remaining > 0.0)
	}

	pub fn begin(&mut self, id: TanzId, _direction: Vec2, cooldown: f32) {
		self.active = Some(ActiveTanz { id, age: 0.0, phase: TanzPhase::Startup, applied: false });
		self.cooldowns.insert(id, cooldown.max(id.definition().timing.cooldown));
	}

	pub fn submit(&mut self, id: TanzId) {
		if self.active.is_none() {
			self.begin(id, Vec2::ZERO, id.definition().timing.cooldown);
			return;
		}
		if self.in_chain_window() {
			self.buffered = Some(BufferedTanz { id, age: 0.0 });
		}
	}

	pub fn in_chain_window(&self) -> bool {
		let Some(active) = self.active else {
			return false;
		};
		let timing = active.id.definition().timing;
		!matches!(active.phase, TanzPhase::Startup)
			&& active.age <= timing.active_end() + timing.chain_window
	}

	pub fn tick(&mut self, dt: f32) {
		for remaining in self.cooldowns.values_mut() {
			*remaining = (*remaining - dt).max(0.0);
		}
		self.cooldowns.retain(|_, remaining| *remaining > 0.0);

		if let Some(buffered) = &mut self.buffered {
			buffered.age += dt;
			if buffered.age > buffered.id.definition().timing.buffer_ttl {
				self.buffered = None;
			}
		}

		let Some(active) = self.active.as_mut() else {
			self.promote_buffered();
			return;
		};
		active.age += dt;
		let timing = active.id.definition().timing;
		let next = if active.age < timing.startup_end() {
			TanzPhase::Startup
		} else if active.age < timing.active_end() {
			TanzPhase::Active
		} else if active.age < timing.total() {
			TanzPhase::Recovery
		} else {
			self.active = None;
			self.promote_buffered();
			return;
		};
		let entered_recovery = active.phase != TanzPhase::Recovery && next == TanzPhase::Recovery;
		active.phase = next;
		if entered_recovery {
			self.promote_buffered();
		}
	}

	fn promote_buffered(&mut self) {
		let Some(buffered) = self.buffered.take() else {
			return;
		};
		self.begin(buffered.id, Vec2::ZERO, buffered.id.definition().timing.cooldown);
	}

	pub fn just_entered_active(&mut self) -> Option<TanzId> {
		let active = self.active.as_mut()?;
		if active.phase == TanzPhase::Active && !active.applied {
			active.applied = true;
			Some(active.id)
		} else {
			None
		}
	}

	pub fn move_local(&self) -> Option<Vec2> {
		match self.move_drive() {
			Some(TanzMoveDrive::Jog(local) | TanzMoveDrive::Burst { local, .. }) => Some(local),
			None => None,
		}
	}

	fn move_drive(&self) -> Option<TanzMoveDrive> {
		let active = self.active.as_ref()?;
		match active.id.definition().action {
			TanzAction::Heal { .. } => None,
			TanzAction::Move { local, cruise } => {
				if cruise > 0.0 {
					let total = active.id.definition().timing.total().max(1e-4);
					let progress = (active.age / total).clamp(0.0, 1.0);
					Some(TanzMoveDrive::Burst { local, speed: dash_ground_speed(progress, cruise) })
				} else if active.phase == TanzPhase::Active {
					Some(TanzMoveDrive::Jog(local))
				} else {
					None
				}
			}
		}
	}
}

enum TanzMoveDrive {
	Jog(Vec2),
	Burst { local: Vec2, speed: f32 },
}

pub fn match_tanz_inputs(
	mut users: Query<(&TanzUser, &TanzStack, &mut TanzExecution)>,
	mut inputs: MessageReader<TanzInput>,
) {
	for input in inputs.read() {
		let Ok((user, stack, mut execution)) = users.get_mut(input.user) else {
			continue;
		};
		if !user.mode.is_tanzer() {
			continue;
		}
		let Some(id) = stack.match_input(input, |id| execution.cooling(id)) else {
			continue;
		};
		execution.submit(id);
	}
}

pub fn advance_tanz_execution(
	time: Res<Time>,
	mut users: Query<(&TanzUser, &mut TanzExecution, Option<&mut Health>)>,
) {
	let dt = time.delta_secs();
	for (user, mut execution, health) in &mut users {
		if user.mode != ControlMode::Tanzer
			&& execution.active.is_none()
			&& execution.buffered.is_none()
		{
			execution.tick(dt);
			continue;
		}
		execution.tick(dt);
		let Some(id) = execution.just_entered_active() else {
			continue;
		};
		if let TanzAction::Heal { amount } = id.definition().action {
			if let Some(mut health) = health {
				health.heal(amount);
			}
		}
	}
}

pub fn apply_tanz_movement(
	mut commands: Commands,
	users: Query<(Entity, &TanzUser, &TanzExecution, Option<&PlayerLook>)>,
	mut wishes: Query<&mut MoveWish>,
) {
	for (entity, user, execution, look) in &users {
		if !user.mode.is_tanzer() {
			continue;
		}
		let Ok(mut wish) = wishes.get_mut(entity) else {
			continue;
		};
		match execution.move_drive() {
			Some(TanzMoveDrive::Jog(local)) => {
				wish.0 = look_wish(look_yaw(look), local);
				commands.entity(entity).remove::<MoveSpeedCap>();
			}
			Some(TanzMoveDrive::Burst { local, speed }) => {
				if speed > 1e-4 {
					wish.0 = look_wish(look_yaw(look), local);
					commands.entity(entity).insert(MoveSpeedCap(speed));
				} else {
					wish.0 = Vec3::ZERO;
					commands.entity(entity).remove::<MoveSpeedCap>();
				}
			}
			None => {
				wish.0 = Vec3::ZERO;
				commands.entity(entity).remove::<MoveSpeedCap>();
			}
		}
	}
}

pub fn drive_tanz_animation(
	mut commands: Commands,
	users: Query<(&TanzUser, &TanzExecution)>,
	visuals: Query<(&CharacterMembers, &ChildOf), With<CharacterRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	for (members, child_of) in &visuals {
		let Ok((user, execution)) = users.get(child_of.parent()) else {
			continue;
		};
		if !user.mode.is_tanzer() {
			continue;
		}
		let Some(active) = execution.active else {
			continue;
		};
		let definition = active.id.definition();
		let desired = AnimRef::new(definition.clip);
		let progress = (active.age / definition.timing.total().max(1e-4)).clamp(0.0, 1.0);
		for member in members.iter() {
			let Ok(rig) = rigs.get(member) else {
				continue;
			};
			if rig.role != CharacterRigRole::Body {
				continue;
			}
			let needs = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs {
				commands.entity(member).insert(AnimRefRoot(desired));
			}
			commands.entity(member).insert(AnimProgress(progress));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::actions::TanzAction;
	use crate::input::TanzButton;
	use crate::TanzUser;

	fn input(user: Entity, direction: Vec2, button: TanzButton) -> TanzInput {
		TanzInput { user, direction, magnitude: 0.9, coherence: 1.0, button }
	}

	#[test]
	fn buffer_replaces_and_chains_at_recovery() -> anyhow::Result<()> {
		let mut execution = TanzExecution::default();
		execution.submit(TanzId::LeftStrafe);
		execution.tick(0.10);
		assert_eq!(execution.active.map(|active| active.phase), Some(TanzPhase::Active));
		execution.submit(TanzId::RightStrafe);
		execution.submit(TanzId::ForwardDash);
		assert_eq!(execution.buffered.map(|buffered| buffered.id), Some(TanzId::ForwardDash));
		execution.tick(0.20);
		assert_eq!(execution.active.map(|active| active.id), Some(TanzId::ForwardDash));
		assert_eq!(execution.active.map(|active| active.phase), Some(TanzPhase::Startup));
		Ok(())
	}

	#[test]
	fn cooldown_blocks_repeat_until_it_expires() -> anyhow::Result<()> {
		let mut execution = TanzExecution::default();
		execution.submit(TanzId::LeftStrafe);
		assert!(execution.cooling(TanzId::LeftStrafe));
		execution.tick(0.10);
		assert!(execution.cooling(TanzId::LeftStrafe));
		execution.tick(0.40);
		assert!(!execution.cooling(TanzId::LeftStrafe));
		Ok(())
	}

	#[test]
	fn healing_salute_restores_health_without_jump() -> anyhow::Result<()> {
		let mut execution = TanzExecution::default();
		let mut health = Health { current: 40.0, max: 100.0 };
		execution.submit(TanzId::HealingSalute);
		execution.tick(0.10);
		if let Some(id) = execution.just_entered_active() {
			if let TanzAction::Heal { amount } = id.definition().action {
				health.heal(amount);
			}
		}
		assert!((health.current - 65.0).abs() < 1e-4, "{}", health.current);
		assert_eq!(execution.active.map(|active| active.id), Some(TanzId::HealingSalute));
		Ok(())
	}

	#[test]
	fn unmatched_input_does_not_start_a_tanz() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<TanzInput>()
			.add_systems(Update, match_tanz_inputs);
		let user = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				TanzStack::starter(),
				TanzExecution::default(),
			))
			.id();
		app.world_mut().write_message(input(user, Vec2::ZERO, TanzButton::APress));
		app.update();
		assert!(app
			.world()
			.get::<TanzExecution>(user)
			.is_some_and(|execution| execution.active.is_none()));
		Ok(())
	}

	#[test]
	fn strafe_writes_a_lateral_wish() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, apply_tanz_movement);
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::LeftStrafe, Vec2::NEG_X, 0.35);
		execution.tick(0.10);
		let user = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				execution,
				MoveWish::default(),
				PlayerLook::default(),
			))
			.id();
		app.update();
		let wish = app.world().get::<MoveWish>(user).ok_or_else(|| anyhow::anyhow!("wish"))?;
		let cap = app
			.world()
			.get::<MoveSpeedCap>(user)
			.ok_or_else(|| anyhow::anyhow!("strafe should set MoveSpeedCap"))?;
		assert!(wish.0.x < -0.9, "{}", wish.0);
		assert!(wish.0.y.abs() < 1e-4);
		assert!(
			(cap.0 - crate::actions::POWER_STRAFE_CRUISE).abs() < 1e-3,
			"glide-adjacent cap {}",
			cap.0
		);
		Ok(())
	}

	#[test]
	fn left_strafe_is_still_during_compress() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, apply_tanz_movement);
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::LeftStrafe, Vec2::NEG_X, 0.35);
		execution.tick(0.02);
		let user = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				execution,
				MoveWish::default(),
				PlayerLook::default(),
			))
			.id();
		app.update();
		let wish = app.world().get::<MoveWish>(user).ok_or_else(|| anyhow::anyhow!("wish"))?;
		assert_eq!(wish.0, Vec3::ZERO);
		assert!(app.world().get::<MoveSpeedCap>(user).is_none());
		Ok(())
	}

	#[test]
	fn forward_dash_writes_burst_cap_during_the_hold() -> anyhow::Result<()> {
		use crate::actions::FORWARD_DASH_CRUISE;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, apply_tanz_movement);
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::ForwardDash, Vec2::Y, 0.35);
		execution.tick(0.15);
		let user = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				execution,
				MoveWish::default(),
				PlayerLook::default(),
			))
			.id();
		app.update();
		let wish = app.world().get::<MoveWish>(user).ok_or_else(|| anyhow::anyhow!("wish"))?;
		let cap = app
			.world()
			.get::<MoveSpeedCap>(user)
			.ok_or_else(|| anyhow::anyhow!("dash should set MoveSpeedCap"))?;
		assert!(wish.0.z.abs() > 0.9, "dash wish should be look-forward, got {}", wish.0);
		assert!((cap.0 - FORWARD_DASH_CRUISE).abs() < 1e-3, "hold cap {}", cap.0);
		Ok(())
	}

	#[test]
	fn forward_dash_is_still_during_compress() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, apply_tanz_movement);
		let mut execution = TanzExecution::default();
		execution.begin(TanzId::ForwardDash, Vec2::Y, 0.35);
		execution.tick(0.02);
		let user = app
			.world_mut()
			.spawn((
				TanzUser { mode: ControlMode::Tanzer, ..TanzUser::default() },
				execution,
				MoveWish::default(),
				PlayerLook::default(),
			))
			.id();
		app.update();
		let wish = app.world().get::<MoveWish>(user).ok_or_else(|| anyhow::anyhow!("wish"))?;
		assert_eq!(wish.0, Vec3::ZERO);
		assert!(app.world().get::<MoveSpeedCap>(user).is_none());
		Ok(())
	}
}
