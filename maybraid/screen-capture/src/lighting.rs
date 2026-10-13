//! Lighting presets that participate in the cache key.

use bevy::prelude::{DirectionalLight, EulerRot, Quat, Transform, Vec3};
use std::f32::consts::PI;

/// Lighting configuration for a shaded capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureLighting {
	pub preset: CaptureLightingPreset,
	/// Bump when the application’s world lighting recipe changes.
	pub revision: u32,
}

/// Named lighting, including an application-supplied world key/fill pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptureLightingPreset {
	/// Two-directional studio rig used by the items playground.
	Studio,
	/// Single overhead fill, no hard key.
	Neutral,
	/// Application world lighting (key + fill).
	World { key: CaptureDirectionalLight, fill: CaptureDirectionalLight },
}

/// Serializable directional light used by [`CaptureLightingPreset::World`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureDirectionalLight {
	pub illuminance: f32,
	pub euler_xyz: [f32; 3],
}

impl CaptureLighting {
	pub const fn studio() -> Self {
		Self { preset: CaptureLightingPreset::Studio, revision: 1 }
	}

	pub const fn neutral() -> Self {
		Self { preset: CaptureLightingPreset::Neutral, revision: 1 }
	}

	pub const fn world(key: CaptureDirectionalLight, fill: CaptureDirectionalLight) -> Self {
		Self { preset: CaptureLightingPreset::World { key, fill }, revision: 1 }
	}

	pub const fn with_revision(mut self, revision: u32) -> Self {
		self.revision = revision;
		self
	}

	pub fn cache_fragment(self) -> String {
		format!("{}#{}", self.preset.cache_fragment(), self.revision)
	}

	/// Isolated capture lights at `origin`. Empty for silhouette (unlit).
	pub fn spawn_specs(self, origin: Vec3) -> Vec<(DirectionalLight, Transform)> {
		self.preset.spawn_specs(origin)
	}
}

impl CaptureLightingPreset {
	pub fn cache_fragment(self) -> String {
		match self {
			Self::Studio => String::from("studio"),
			Self::Neutral => String::from("neutral"),
			Self::World { key, fill } => {
				format!("world:{}+{}", key.cache_fragment(), fill.cache_fragment())
			}
		}
	}

	fn spawn_specs(self, origin: Vec3) -> Vec<(DirectionalLight, Transform)> {
		match self {
			Self::Studio => vec![
				directional(10000.0, [-PI / 4.0, PI / 4.0, 0.0], origin, true),
				directional(500.0, [PI / 4.0, -PI / 4.0, 0.0], origin, false),
			],
			Self::Neutral => {
				vec![directional(2500.0, [-PI / 2.2, 0.0, 0.0], origin, false)]
			}
			Self::World { key, fill } => vec![
				directional(key.illuminance, key.euler_xyz, origin, true),
				directional(fill.illuminance, fill.euler_xyz, origin, false),
			],
		}
	}
}

impl CaptureDirectionalLight {
	pub const fn new(illuminance: f32, euler_xyz: [f32; 3]) -> Self {
		Self { illuminance, euler_xyz }
	}

	fn cache_fragment(self) -> String {
		format!(
			"{}/{}/{}/{}",
			self.illuminance.to_bits(),
			self.euler_xyz[0].to_bits(),
			self.euler_xyz[1].to_bits(),
			self.euler_xyz[2].to_bits(),
		)
	}
}

fn directional(
	illuminance: f32,
	euler_xyz: [f32; 3],
	origin: Vec3,
	shadows: bool,
) -> (DirectionalLight, Transform) {
	(
		DirectionalLight {
			illuminance,
			shadow_maps_enabled: shadows,
			..DirectionalLight::default()
		},
		Transform::from_translation(origin + Vec3::Y).with_rotation(Quat::from_euler(
			EulerRot::XYZ,
			euler_xyz[0],
			euler_xyz[1],
			euler_xyz[2],
		)),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_preset_changes_cache_key() -> anyhow::Result<()> {
		let studio = CaptureLighting::studio();
		let world = CaptureLighting::world(
			CaptureDirectionalLight::new(8000.0, [-0.8, 0.4, 0.0]),
			CaptureDirectionalLight::new(400.0, [0.6, -0.3, 0.0]),
		);
		assert_ne!(studio.cache_fragment(), world.cache_fragment());
		assert_ne!(studio.with_revision(2).cache_fragment(), studio.cache_fragment());
		Ok(())
	}
}
