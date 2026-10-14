//! Rolled blast, damage, and throw speed for generated grenades.

use crate::firearm_roll::Dist;
use crate::names::mix;
use crate::{GrenadeSpec, GrenadeStats, ItemRng, STANDARD_LAUNCH_SPEED};

const GRENADE_SEED: u64 = 0x6E4A_DE00_F1A4_0002;

/// Inclusive authored band. Matches the visible fireball / smoke swell, not
/// the smaller bright core used on the first street-scale pass.
pub const BLAST_RADIUS_MIN: f32 = 8.0;
pub const BLAST_RADIUS_MAX: f32 = 13.0;
pub const BLAST_DAMAGE_MIN: f32 = 50.0;
pub const BLAST_DAMAGE_MAX: f32 = 150.0;
pub const LAUNCH_SPEED_MIN: f32 = 15.0;
pub const LAUNCH_SPEED_MAX: f32 = 30.0;

pub fn generate_grenade_stats(spec: &GrenadeSpec) -> GrenadeStats {
	let mut rng = ItemRng::from_seed(mix(GRENADE_SEED, spec.mesh.label()));
	realize_grenade_stats(&mut rng)
}

pub fn realize_grenade_stats(rng: &mut ItemRng) -> GrenadeStats {
	let mut stats = GrenadeStats::standard();
	stats.blast_radius =
		Dist::new(10.5, 1.3).sample_clamped(rng, BLAST_RADIUS_MIN, BLAST_RADIUS_MAX);
	stats.blast_damage =
		Dist::new(95.0, 22.0).sample_clamped(rng, BLAST_DAMAGE_MIN, BLAST_DAMAGE_MAX);
	stats.launch_speed = Dist::new(STANDARD_LAUNCH_SPEED, 4.0).sample_clamped(
		rng,
		LAUNCH_SPEED_MIN,
		LAUNCH_SPEED_MAX,
	);
	stats.effect_scale = effect_scale_for_blast(stats.blast_radius);
	stats
}

/// Fiery-explosion scale `1` reads as about this many meters of fireball plus
/// smoke swell. A 10.5 m blast at scale 13 fills that visible ring.
pub const FIERY_EXPLOSION_METERS: f32 = 10.5 / 13.0;

pub fn effect_scale_for_blast(blast_radius: f32) -> f32 {
	(blast_radius / FIERY_EXPLOSION_METERS).clamp(1.0, 20.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn realized_blast_stays_in_the_wide_band() {
		let mut rng = ItemRng::from_seed(11);
		for _ in 0..40 {
			let stats = realize_grenade_stats(&mut rng);
			assert!(
				(BLAST_RADIUS_MIN..=BLAST_RADIUS_MAX).contains(&stats.blast_radius),
				"{}",
				stats.blast_radius
			);
			assert!(
				(BLAST_DAMAGE_MIN..=BLAST_DAMAGE_MAX).contains(&stats.blast_damage),
				"{}",
				stats.blast_damage
			);
			assert!(
				(LAUNCH_SPEED_MIN..=LAUNCH_SPEED_MAX).contains(&stats.launch_speed),
				"{}",
				stats.launch_speed
			);
			assert!(
				(stats.effect_scale - stats.blast_radius / FIERY_EXPLOSION_METERS).abs() < 1e-4,
				"scale {} radius {}",
				stats.effect_scale,
				stats.blast_radius
			);
		}
	}

	#[test]
	fn realized_launch_speed_varies_inside_the_band() {
		let mut rng = ItemRng::from_seed(11);
		let mut speeds = Vec::new();
		for _ in 0..40 {
			speeds.push(realize_grenade_stats(&mut rng).launch_speed);
		}
		let min = speeds.iter().copied().fold(f32::INFINITY, f32::min);
		let max = speeds.iter().copied().fold(f32::NEG_INFINITY, f32::max);
		assert!(min < STANDARD_LAUNCH_SPEED);
		assert!(max > STANDARD_LAUNCH_SPEED);
		assert!(min >= LAUNCH_SPEED_MIN);
		assert!(max <= LAUNCH_SPEED_MAX);
	}

	#[test]
	fn effect_scale_matches_street_blast_meters() {
		assert!((effect_scale_for_blast(10.5) - 13.0).abs() < 1e-4);
		assert!(effect_scale_for_blast(10.5) > 4.0);
	}

	#[test]
	fn generate_is_stable_for_a_spec() {
		let spec = GrenadeSpec::standard();
		assert_eq!(generate_grenade_stats(&spec), generate_grenade_stats(&spec));
	}
}
