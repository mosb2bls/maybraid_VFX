//! Spatial rumble: distance-scaled camera, ground, and optional pad feedback.
//!
//! Any source can dispatch [`SpatialRumble`]. The plugin does not depend on a
//! character, encounter, or gameplay hazard.

mod channels;
mod compose;
mod listener;
mod math;
mod present;
mod source;

pub use channels::{
	CameraRumbleShape, GroundRumbleSample, GroundRumbleShape, PadRumbleShape, RumbleChannels,
};
pub use compose::{
	compose_output, SpatialRumbleOutput, SpatialRumbleSettings, SpatialRumbleSystems,
};
pub use listener::{AppliedCameraRumble, SpatialRumbleListener, SpatialRumbleReceiver};
pub use math::{RumbleEnvelope, RumbleFalloff};
pub use present::apply_camera_shake;
pub use source::{SpatialRumble, SpatialRumbleSource};

use bevy::prelude::*;

use crate::compose::{compose_rumbles, ingest_rumbles, tick_rumbles};
use crate::present::{cleanup_removed_listeners, present_camera_rumble};

/// Registers ingest / tick / compose / present. Pad pulses use existing
/// [`maybraid_input::PadRumble`]; keyboard and mouse need no extra path.
pub struct SpatialRumblePlugin;

impl Plugin for SpatialRumblePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<SpatialRumbleSettings>()
			.init_resource::<SpatialRumbleOutput>()
			.add_message::<SpatialRumble>();
		if !app.world().contains_resource::<Messages<maybraid_input::PadRumble>>() {
			app.add_message::<maybraid_input::PadRumble>();
		}
		app.configure_sets(
			Update,
			(
				SpatialRumbleSystems::Ingest,
				SpatialRumbleSystems::Tick,
				SpatialRumbleSystems::Compose,
				SpatialRumbleSystems::Present,
			)
				.chain(),
		)
		.add_systems(Update, ingest_rumbles.in_set(SpatialRumbleSystems::Ingest))
		.add_systems(Update, tick_rumbles.in_set(SpatialRumbleSystems::Tick))
		.add_systems(Update, compose_rumbles.in_set(SpatialRumbleSystems::Compose))
		.add_systems(
			Update,
			(present_camera_rumble, cleanup_removed_listeners)
				.chain()
				.in_set(SpatialRumbleSystems::Present),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::math::RumbleEnvelope;
	use anyhow::Result;
	use bevy::time::TimeUpdateStrategy;
	use std::time::Duration;

	fn app_with_time(dt: f32) -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins(SpatialRumblePlugin)
			.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(dt)));
		app
	}

	fn spawn_listener(app: &mut App, translation: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Transform::from_translation(translation),
				GlobalTransform::from_translation(translation),
				SpatialRumbleListener::default(),
			))
			.id()
	}

	fn live_source_count(app: &mut App) -> usize {
		let mut query = app.world_mut().query::<&SpatialRumbleSource>();
		query.iter(app.world()).count()
	}

	fn first_source_age(app: &mut App) -> Result<f32> {
		let mut query = app.world_mut().query::<&SpatialRumbleSource>();
		query
			.iter(app.world())
			.next()
			.map(|source| source.age)
			.ok_or_else(|| anyhow::anyhow!("source"))
	}

	fn pump(app: &mut App) {
		app.update();
		app.update();
	}

	#[test]
	fn dispatched_effect_scales_with_distance() -> Result<()> {
		let mut near = app_with_time(1.0 / 60.0);
		spawn_listener(&mut near, Vec3::ZERO);
		near.world_mut()
			.write_message(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(9));
		pump(&mut near);
		let near_amp = near.world().resource::<SpatialRumbleOutput>().ground.amplitude;

		let mut far = app_with_time(1.0 / 60.0);
		spawn_listener(&mut far, Vec3::X * 20.0);
		far.world_mut()
			.write_message(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(9));
		pump(&mut far);
		let far_out = far.world().resource::<SpatialRumbleOutput>();
		assert!(near_amp > far_out.ground.amplitude);
		assert!(far_out.ground.is_silent());
		Ok(())
	}

	#[test]
	fn overlapping_effects_stay_bounded_and_clear() -> Result<()> {
		let mut app = app_with_time(0.05);
		let camera = spawn_listener(&mut app, Vec3::ZERO);
		app.world_mut().insert_resource(SpatialRumbleSettings {
			max_camera_offset: 0.05,
			max_sources: 2,
			..SpatialRumbleSettings::default()
		});
		app.world_mut().write_message(SpatialRumble::bray(Vec3::ZERO).with_seed(1));
		app.world_mut()
			.write_message(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(2));
		app.world_mut().write_message(SpatialRumble::bray(Vec3::ZERO).with_seed(3));
		app.update();
		assert!(live_source_count(&mut app) <= 2);
		let offset = app.world().resource::<SpatialRumbleOutput>().camera_offset.length();
		assert!(offset <= 0.05 + 1e-5);

		for _ in 0..40 {
			app.update();
		}
		assert_eq!(live_source_count(&mut app), 0);
		assert!(app.world().resource::<SpatialRumbleOutput>().is_quiet());
		let transform = *app.world().get::<Transform>(camera).ok_or(anyhow::anyhow!("camera"))?;
		assert!(transform.translation.length() < 1e-4);
		Ok(())
	}

	#[test]
	fn lifetime_expires_and_cleans_applied_offset() -> Result<()> {
		let mut app = app_with_time(0.1);
		let camera = spawn_listener(&mut app, Vec3::Y);
		app.world_mut().write_message(
			SpatialRumble {
				origin: Vec3::ZERO,
				intensity: 1.0,
				duration: 0.2,
				envelope: RumbleEnvelope::new(0.0, 0.0),
				seed: Some(11),
				..SpatialRumble::default()
			}
			.with_seed(11),
		);
		app.update();
		assert_eq!(live_source_count(&mut app), 1);
		let shaken = *app.world().get::<Transform>(camera).ok_or(anyhow::anyhow!("camera"))?;
		assert!((shaken.translation - Vec3::Y).length() > 1e-5);

		app.update();
		app.update();
		assert_eq!(live_source_count(&mut app), 0);
		let restored = *app.world().get::<Transform>(camera).ok_or(anyhow::anyhow!("camera"))?;
		assert!((restored.translation - Vec3::Y).length() < 1e-4);
		let applied = app.world().get::<AppliedCameraRumble>(camera).copied().unwrap_or_default();
		assert!(applied.is_zero());
		Ok(())
	}

	#[test]
	fn replacing_the_listener_clears_output() -> Result<()> {
		let mut app = app_with_time(1.0 / 60.0);
		let first = spawn_listener(&mut app, Vec3::ZERO);
		app.world_mut().write_message(SpatialRumble::bray(Vec3::ZERO).with_seed(5));
		pump(&mut app);
		assert!(!app.world().resource::<SpatialRumbleOutput>().is_quiet());

		app.world_mut().entity_mut(first).remove::<SpatialRumbleListener>();
		let second = spawn_listener(&mut app, Vec3::new(0.0, 1.0, 0.0));
		app.update();
		let applied = app.world().get::<AppliedCameraRumble>(second).copied().unwrap_or_default();
		assert!(applied.translation.length() < 0.3);
		let first_transform =
			*app.world().get::<Transform>(first).ok_or(anyhow::anyhow!("first"))?;
		assert!(first_transform.translation.length() < 0.05);
		Ok(())
	}

	#[test]
	fn keyboard_play_does_not_require_a_gamepad() -> Result<()> {
		let mut app = app_with_time(1.0 / 60.0);
		spawn_listener(&mut app, Vec3::ZERO);
		app.world_mut().write_message(SpatialRumble::bray(Vec3::ZERO).with_seed(8));
		pump(&mut app);
		assert_eq!(live_source_count(&mut app), 1);
		assert!(app.world().resource::<SpatialRumbleOutput>().ground.amplitude > 0.0);
		Ok(())
	}

	#[test]
	fn pause_freezes_source_age() -> Result<()> {
		let mut app = app_with_time(0.05);
		spawn_listener(&mut app, Vec3::ZERO);
		app.world_mut().write_message(
			SpatialRumble {
				duration: 1.0,
				envelope: RumbleEnvelope::new(0.0, 0.0),
				seed: Some(2),
				..SpatialRumble::bray(Vec3::ZERO)
			}
			.with_seed(2),
		);
		app.update();
		let age_before = first_source_age(&mut app)?;
		app.world_mut().resource_mut::<Time<Virtual>>().pause();
		app.update();
		let age_after = first_source_age(&mut app)?;
		assert!((age_after - age_before).abs() < 1e-5);
		Ok(())
	}
}
