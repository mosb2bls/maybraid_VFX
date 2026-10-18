use std::sync::OnceLock;

static SEGMENT_DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();

/// Whether jump-segment debug logging is enabled.
///
/// Matches `std::env::var("CROZON_ANIMATION_DEBUG").is_ok()` (any value, including empty,
/// enables debug; unset disables it). The result is resolved once on first use.
pub fn segment_debug_enabled() -> bool {
	*SEGMENT_DEBUG_ENABLED.get_or_init(segment_debug_enabled_from_env)
}

fn segment_debug_enabled_from_env() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::{TwoFootedJump, TwoFootedTuckedFlip};
	use crate::Animation;

	fn segment_debug_enabled_legacy() -> bool {
		std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
	}

	#[test]
	fn segment_debug_truthiness_matches_legacy_check() {
		assert_eq!(
			segment_debug_enabled_from_env(),
			segment_debug_enabled_legacy(),
			"startup truthiness must match direct env::var().is_ok()"
		);
	}

	#[test]
	fn segment_debug_cache_matches_legacy_on_first_read() {
		assert_eq!(segment_debug_enabled(), segment_debug_enabled_legacy());
	}

	#[test]
	fn jump_pose_unchanged_when_debug_env_set() -> anyhow::Result<()> {
		let jump = TwoFootedJump::default();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		let elapsed = jump.timings(lengths).air_end() + 0.05;

		let mut baseline = HumanoidV0Rig::imported();
		jump.apply(&mut baseline, elapsed);

		let mut with_debug = HumanoidV0Rig::imported();
		// Logging branch only; pose path must not depend on the flag.
		let _ = segment_debug_enabled();
		jump.apply(&mut with_debug, elapsed);

		for bone in ["femur.L", "shin.L", "shoulder.L"] {
			assert_eq!(baseline.posed_angle(bone), with_debug.posed_angle(bone), "{bone}");
		}
		Ok(())
	}

	#[test]
	fn tucked_flip_effects_unchanged_when_debug_env_set() -> anyhow::Result<()> {
		let flip = TwoFootedTuckedFlip::default();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		let timings = flip.timings(lengths);
		let elapsed = timings.air_end() + timings.land_descent_duration * 0.25;

		let mut baseline = HumanoidV0Rig::imported();
		let baseline_effects = flip.apply(&mut baseline, elapsed);

		let mut with_debug = HumanoidV0Rig::imported();
		let _ = segment_debug_enabled();
		let debug_effects = flip.apply(&mut with_debug, elapsed);

		assert_eq!(baseline_effects, debug_effects);
		Ok(())
	}

	/// Simulates Fall/Land sampling calling the debug gate each frame.
	#[test]
	#[ignore]
	fn segment_debug_gate_microbench() {
		const CHARACTERS: usize = 32;
		const FRAMES: usize = 5000;
		const RUNS: usize = 5;

		fn sample_legacy() {
			for _ in 0..CHARACTERS {
				for _ in 0..FRAMES {
					black_box(std::env::var("CROZON_ANIMATION_DEBUG").is_ok());
				}
			}
		}

		fn sample_cached() {
			for _ in 0..CHARACTERS {
				for _ in 0..FRAMES {
					black_box(segment_debug_enabled());
				}
			}
		}

		fn median_ns(times: &[u128]) -> u128 {
			let mut sorted = times.to_vec();
			sorted.sort_unstable();
			sorted[sorted.len() / 2]
		}

		let mut legacy_times = Vec::with_capacity(RUNS);
		let mut cached_times = Vec::with_capacity(RUNS);

		for _ in 0..RUNS {
			let start = Instant::now();
			sample_legacy();
			legacy_times.push(start.elapsed().as_nanos());
		}
		for _ in 0..RUNS {
			let start = Instant::now();
			sample_cached();
			cached_times.push(start.elapsed().as_nanos());
		}

		let legacy_median = median_ns(&legacy_times);
		let cached_median = median_ns(&cached_times);
		let samples = CHARACTERS * FRAMES;
		eprintln!(
			"segment_debug_gate_microbench: characters={} frames={} runs={}",
			CHARACTERS, FRAMES, RUNS
		);
		eprintln!(
			"  legacy env::var median: {} ns ({} ns/sample)",
			legacy_median,
			legacy_median / samples as u128
		);
		eprintln!(
			"  cached OnceLock median: {} ns ({} ns/sample)",
			cached_median,
			cached_median / samples as u128
		);
	}
}
