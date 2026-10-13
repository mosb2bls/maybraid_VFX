//! Push [`SkyClock`] onto lights, blue dome, cosmos, moon, and distance fog.
//!
//! A paused clock only writes when the phase or dome settings change, so
//! golden-hour play does not dirty materials every frame.

use bevy::prelude::*;

use crate::celestial::SkyFill;
use crate::clock::SkyClock;
use crate::dome::{DomeSettings, SkyDomeMaterial, SkyWash};
use crate::field::{SkyField, SkyFieldMaterial};
use crate::SkySun;

pub(crate) fn apply_sky_mood(
	time: Res<Time>,
	mut clock: ResMut<SkyClock>,
	settings: Res<DomeSettings>,
	mut ambient: Option<ResMut<GlobalAmbientLight>>,
	mut sun: Query<(&mut Transform, &mut DirectionalLight), With<SkySun>>,
	mut fill: Query<&mut DirectionalLight, (With<SkyFill>, Without<SkySun>)>,
	field: Query<&MeshMaterial3d<SkyFieldMaterial>, With<SkyField>>,
	mut field_mats: ResMut<Assets<SkyFieldMaterial>>,
	wash: Query<&MeshMaterial3d<SkyDomeMaterial>, With<SkyWash>>,
	mut wash_mats: ResMut<Assets<SkyDomeMaterial>>,
	mut fog: Query<&mut DistanceFog>,
) {
	if !clock.paused {
		clock.advance(time.delta_secs());
	}
	if !clock.is_changed() && !settings.is_changed() {
		return;
	}

	let mood = clock.sample();
	let pose = mood.sun_pose();

	if let Some(ambient) = ambient.as_mut() {
		ambient.brightness = mood.ambient;
	}

	for (mut transform, mut light) in &mut sun {
		*transform = pose;
		light.color = mood.sun_color;
		light.illuminance = mood.sun_illuminance;
	}
	for mut light in &mut fill {
		light.color = mood.fill_color;
		light.illuminance = mood.fill_illuminance;
	}

	let field_params = SkyFieldMaterial::from_mood(mood).params;
	for handle in &field {
		let Some(current) = field_mats.get(&handle.0) else {
			continue;
		};
		if current.params == field_params {
			continue;
		}
		if let Some(mut material) = field_mats.get_mut(&handle.0) {
			material.params = field_params;
		}
	}

	let wash_params = SkyDomeMaterial::from_mood(mood, settings.max_alpha).params;
	for handle in &wash {
		let Some(current) = wash_mats.get(&handle.0) else {
			continue;
		};
		if current.params == wash_params {
			continue;
		}
		if let Some(mut material) = wash_mats.get_mut(&handle.0) {
			material.params = wash_params;
		}
	}

	for mut fog in &mut fog {
		fog.color = mood.fog;
		let mut sun_fog = mood.sun_color.to_linear();
		sun_fog.alpha = 0.4 * mood.day_weight;
		fog.directional_light_color = Color::from(sun_fog);
		// Night pulls the far plane in so distant terrain does not stay readable.
		let day = mood.day_weight;
		fog.falloff =
			FogFalloff::Linear { start: 180.0 + 520.0 * day, end: 1_100.0 + 3_400.0 * day };
	}
}
