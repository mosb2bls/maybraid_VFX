//! Holster pose and rustle. The holster clock lives in `maybraid-equipment`.

use bevy::prelude::*;
use maybraid_audio::{Audio, AudioClip, Mixer, MovementSounds};
use maybraid_equipment::EquipmentHolster;

use crate::pose::HeldFirearm;
use crate::FirearmUser;

pub use maybraid_equipment::{
	EquipmentHolster as WeaponSwap, EQUIPMENT_HOLSTER_SECS as WEAPON_SWAP_SECS,
};

pub fn apply_weapon_swap_pose(
	users: Query<(&FirearmUser, &EquipmentHolster)>,
	mut guns: Query<&mut Transform, With<HeldFirearm>>,
) {
	for (user, holster) in &users {
		let Ok(mut transform) = guns.get_mut(user.held) else {
			continue;
		};
		holster.apply_to(&mut transform);
	}
}

/// Rustle when the holster / raise window starts.
pub fn play_change_item(
	mut commands: Commands,
	started: Query<&GlobalTransform, Added<EquipmentHolster>>,
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
