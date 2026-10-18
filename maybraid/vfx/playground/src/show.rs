//! Apply queued `/show` requests through the crate spawn helper.

use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use maybraid_vfx::{spawn_vfx, VfxLibrary, VfxSpawn};

use crate::commands::{LoopEnabled, PendingVfxShows, PreviewFreeze, SpreadOnce, VfxShowRequest};

pub const SHOW_ORIGIN: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Resource, Default, Clone, Debug)]
pub struct LastVfxShow {
	pub label: String,
	pub request: Option<VfxShowRequest>,
}

/// Replay the last `/show` so a one-shot burst can be judged.
#[derive(Resource, Default, Clone, Debug)]
pub struct RepeatingVfxShow {
	pub request: Option<VfxShowRequest>,
	pub wait: f32,
}

pub fn apply_pending_shows(
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	mut pending: ResMut<PendingVfxShows>,
	mut last: ResMut<LastVfxShow>,
	mut repeating: ResMut<RepeatingVfxShow>,
	mut spread: ResMut<SpreadOnce>,
	mut status: ResMut<GameCommandStatusText>,
	looping: Res<LoopEnabled>,
) {
	let Some(library) = library else {
		return;
	};
	if spread.0 {
		spread.0 = false;
		if let Some(base) = last.request.clone() {
			pending.0.extend([
				VfxShowRequest { distance: 0.0, seed: base.seed, ..base.clone() },
				VfxShowRequest {
					distance: 6.0,
					seed: base.seed.map(|s| s.wrapping_add(1)),
					..base.clone()
				},
				VfxShowRequest {
					distance: 14.0,
					seed: base.seed.map(|s| s.wrapping_add(2)),
					..base
				},
			]);
		}
	}
	for request in pending.0.drain(..) {
		let Some(definition) = library.get(&request.effect) else {
			continue;
		};
		let spawn = burst(&mut commands, definition, &request);
		if looping.0 {
			repeating.request = Some(VfxShowRequest { seed: spawn.seed, ..request.clone() });
			repeating.wait = 0.0;
		}
		last.request = Some(VfxShowRequest { seed: spawn.seed, ..request.clone() });
		last.label = format!(
			"{} · scale {:.2} · intensity {:.2} · playback {:.2} · d {:.1} · seed {}",
			definition.name,
			request.scale,
			request.intensity,
			request.playback,
			request.distance,
			spawn.resolved_seed()
		);
		status.0 = format!("show {}", last.label);
	}
}

pub fn repeat_shown_effect(
	mut commands: Commands,
	time: Res<Time>,
	library: Option<Res<VfxLibrary>>,
	looping: Res<LoopEnabled>,
	freeze: Res<PreviewFreeze>,
	mut repeating: ResMut<RepeatingVfxShow>,
) {
	if !looping.0 || freeze.age.is_some() {
		return;
	}
	let Some(library) = library else {
		return;
	};
	let Some(request) = repeating.request.clone() else {
		return;
	};
	let Some(definition) = library.get(&request.effect) else {
		return;
	};
	repeating.wait += time.delta_secs();
	let playback = request.playback.clamp(maybraid_vfx::MIN_PLAYBACK, maybraid_vfx::MAX_PLAYBACK);
	if repeating.wait >= definition.duration() / playback + 0.45 {
		repeating.wait = 0.0;
		burst(&mut commands, definition, &request);
	}
}

pub fn pause_at_frozen_age(
	freeze: Res<PreviewFreeze>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	mut time: ResMut<Time<Virtual>>,
) {
	let Some(target) = freeze.age else {
		return;
	};
	if time.is_paused() {
		return;
	}
	if instances.iter().any(|instance| instance.armed && instance.age + 1e-3 >= target) {
		time.pause();
	}
}

fn burst(
	commands: &mut Commands,
	definition: &maybraid_vfx::EffectDefinition,
	request: &VfxShowRequest,
) -> VfxSpawn {
	let spawn = VfxSpawn {
		transform: Transform::from_translation(SHOW_ORIGIN + Vec3::Z * request.distance),
		scale: request.scale,
		intensity: request.intensity,
		seed: request.seed,
		playback: request.playback,
		..default()
	}
	.resolved_for(definition);
	spawn_vfx(commands, definition, spawn.clone());
	spawn
}
