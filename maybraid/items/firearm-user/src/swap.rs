//! Lower the held kit, then raise the next one, while a swap is in flight.

use bevy::prelude::*;
use maybraid_audio::{Audio, AudioClip, Mixer, MovementSounds};

use crate::pose::HeldFirearm;
use crate::FirearmUser;

/// Total holster + raise window. Midpoint is when the kit should change.
pub const WEAPON_SWAP_SECS: f32 = 0.42;

const SWAP_PITCH: f32 = 0.95;
const SWAP_DROP: f32 = 0.22;
const SWAP_INSET: f32 = 0.08;

/// In-progress weapon cycle. Lives on the user so it survives the kit replace.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WeaponSwap {
	pub elapsed: f32,
	pub duration: f32,
	pub swapped: bool,
}

impl Default for WeaponSwap {
	fn default() -> Self {
		Self::start()
	}
}

impl WeaponSwap {
	pub fn start() -> Self {
		Self { elapsed: 0.0, duration: WEAPON_SWAP_SECS, swapped: false }
	}

	pub fn t(self) -> f32 {
		if self.duration <= 0.0 {
			return 1.0;
		}
		(self.elapsed / self.duration).clamp(0.0, 1.0)
	}

	/// 0 ready, 1 lowest (midpoint), 0 ready again.
	pub fn dip(self) -> f32 {
		(self.t() * std::f32::consts::PI).sin()
	}

	pub fn ready_to_swap(self) -> bool {
		!self.swapped && self.elapsed >= self.duration * 0.5
	}

	pub fn finished(self) -> bool {
		self.swapped && self.elapsed >= self.duration
	}

	pub fn mark_swapped(&mut self) {
		self.swapped = true;
	}

	/// Dip the posed kit down and in toward the body.
	pub fn apply_to(self, transform: &mut Transform) {
		let dip = self.dip();
		if dip <= 0.0 {
			return;
		}
		transform.rotation *= Quat::from_rotation_x(dip * SWAP_PITCH);
		transform.translation += Vec3::Y * (-SWAP_DROP * dip);
		transform.translation += transform.rotation * Vec3::new(SWAP_INSET * dip, 0.0, -0.04 * dip);
	}
}

pub fn advance_weapon_swap(time: Res<Time>, mut swaps: Query<&mut WeaponSwap>) {
	let dt = time.delta_secs();
	for mut swap in &mut swaps {
		swap.elapsed += dt;
	}
}

pub fn apply_weapon_swap_pose(
	users: Query<(&FirearmUser, &WeaponSwap)>,
	mut guns: Query<&mut Transform, With<HeldFirearm>>,
) {
	for (user, swap) in &users {
		let Ok(mut transform) = guns.get_mut(user.held) else {
			continue;
		};
		swap.apply_to(&mut transform);
	}
}

pub fn clear_finished_weapon_swaps(mut commands: Commands, swaps: Query<(Entity, &WeaponSwap)>) {
	for (entity, swap) in &swaps {
		if swap.finished() {
			commands.entity(entity).remove::<WeaponSwap>();
		}
	}
}

/// Rustle when the holster / raise window starts.
pub fn play_change_item(
	mut commands: Commands,
	started: Query<&GlobalTransform, Added<WeaponSwap>>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<MovementSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	for transform in &started {
		sounds.play_change_item(
			&mut commands,
			&clips,
			audio,
			&mut mixer,
			listener,
			transform.translation(),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn dip_peaks_at_the_midpoint() {
		let start = WeaponSwap::start();
		assert_eq!(start.dip(), 0.0);
		let mid = WeaponSwap {
			elapsed: WEAPON_SWAP_SECS * 0.5,
			duration: WEAPON_SWAP_SECS,
			swapped: false,
		};
		assert!((mid.dip() - 1.0).abs() < 1e-5);
		assert!(mid.ready_to_swap());
		let done =
			WeaponSwap { elapsed: WEAPON_SWAP_SECS, duration: WEAPON_SWAP_SECS, swapped: true };
		assert!(done.dip().abs() < 1e-5);
		assert!(done.finished());
		assert!(!done.ready_to_swap());
		assert!(!WeaponSwap {
			elapsed: WEAPON_SWAP_SECS,
			duration: WEAPON_SWAP_SECS,
			swapped: false
		}
		.finished());
	}

	#[test]
	fn apply_to_lowers_the_bore() {
		let mut transform = Transform::IDENTITY;
		WeaponSwap { elapsed: WEAPON_SWAP_SECS * 0.5, duration: WEAPON_SWAP_SECS, swapped: false }
			.apply_to(&mut transform);
		assert!(transform.translation.y < 0.0);
		assert!((transform.rotation * Vec3::Z).y < 0.0);
	}
}
