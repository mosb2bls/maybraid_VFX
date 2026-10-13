//! Stamp [`FlinchProfile`] on hurt characters and play a grunt on damage.

use bevy::prelude::*;
use damage::{DamageApplied, Health};
use maybraid_audio::{
	Audio, AudioClip, AudioSystems, FlinchProfile, FlinchSounds, FlinchState, Mixer,
};

use crate::{Npc, Player};

pub(crate) fn ensure_flinch_profiles(
	mut commands: Commands,
	bodies: Query<Entity, (With<Health>, Without<FlinchProfile>, Or<(With<Player>, With<Npc>)>)>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(FlinchProfile::default());
	}
}

pub(crate) fn play_flinch_grunts(
	mut commands: Commands,
	mut applied: MessageReader<DamageApplied>,
	mut profiles: Query<(&FlinchProfile, Option<&mut FlinchState>)>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<FlinchSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	for event in applied.read() {
		if event.amount <= 0.0 {
			continue;
		}
		let Ok((profile, state)) = profiles.get_mut(event.target) else {
			continue;
		};
		let profile = *profile;
		let mut local = state
			.as_deref()
			.copied()
			.unwrap_or_else(|| FlinchState::seeded(event.target.to_bits()));
		sounds.play(
			&mut commands,
			profile,
			&mut local,
			&clips,
			audio,
			&mut mixer,
			listener,
			event.point,
		);
		if let Some(mut state) = state {
			*state = local;
		} else {
			if let Ok(mut entity) = commands.get_entity(event.target) {
				entity.try_insert(local);
			}
		}
	}
}

pub(crate) fn configure_flinch(app: &mut App) {
	app.add_message::<DamageApplied>().add_systems(
		PostUpdate,
		(ensure_flinch_profiles, play_flinch_grunts)
			.chain()
			.after(damage::DamageSystems::Apply)
			.before(AudioSystems::Sync),
	);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn player_with_health_gets_a_man_flinch() {
		let mut app = App::new();
		app.add_systems(Update, ensure_flinch_profiles);
		let player = app.world_mut().spawn((Player, Health::default())).id();
		app.update();
		let profile = app.world().get::<FlinchProfile>(player);
		assert_eq!(profile.map(|profile| profile.grunt), Some(maybraid_audio::GruntStyle::Man));
	}

	#[test]
	fn bodies_without_health_stay_quiet() {
		let mut app = App::new();
		app.add_systems(Update, ensure_flinch_profiles);
		let player = app.world_mut().spawn(Player).id();
		app.update();
		assert!(app.world().get::<FlinchProfile>(player).is_none());
	}
}
