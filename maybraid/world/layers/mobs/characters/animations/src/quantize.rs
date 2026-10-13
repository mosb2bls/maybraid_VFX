//! Quantize continuous clip time onto nearest sample bins.
//!
//! Playback advances continuously. Sampling snaps to the nearest bin and evaluates
//! at that bin's **canonical** time. Interpolation is not applied here.

/// Default clip-time bin width: 10 ms.
pub const DEFAULT_SAMPLE_INTERVAL_US: u32 = 10_000;

/// How clip time maps onto a finite sample table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClipTimePolicy {
	/// Looping clip. Time wraps in `[0, duration)`. The end is identified with 0.
	Cycle { duration: f32 },
	/// One-shot clip. Time clamps to `[0, duration]` and both endpoints stay exact.
	Clamp { duration: f32 },
	/// Unbounded time (idle oscillators). Bins march with clip time; the cache bounds memory.
	Unbounded,
}

/// Nearest cached sample for a clip time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleAddress {
	pub index: u32,
	pub canonical_time: f32,
}

impl ClipTimePolicy {
	/// Finite table length for this policy, if the clip can be precomputed.
	pub fn table_len(self, sample_interval_us: u32) -> Option<usize> {
		let interval = interval_seconds(sample_interval_us);
		match self {
			Self::Cycle { duration } => Some(bins_in_span(duration, interval) as usize),
			Self::Clamp { duration } => Some(bins_in_span(duration, interval) as usize + 1),
			Self::Unbounded => None,
		}
	}
}

impl SampleAddress {
	/// Resolve `clip_time` to the nearest bin of `sample_interval_us`.
	pub fn from_clip_time(clip_time: f32, sample_interval_us: u32, policy: ClipTimePolicy) -> Self {
		let interval = interval_seconds(sample_interval_us);
		match policy {
			ClipTimePolicy::Cycle { duration } => cycle_address(clip_time, interval, duration),
			ClipTimePolicy::Clamp { duration } => clamp_address(clip_time, interval, duration),
			ClipTimePolicy::Unbounded => unbounded_address(clip_time, interval),
		}
	}
}

/// Seconds per bin. A zero interval falls back to the 10 ms default.
pub fn interval_seconds(sample_interval_us: u32) -> f32 {
	let us = if sample_interval_us == 0 { DEFAULT_SAMPLE_INTERVAL_US } else { sample_interval_us };
	us as f32 / 1_000_000.0
}

/// Map `-0.0` to `+0.0` so parameter identity does not split on sign of zero.
pub fn normalize_signed_zero(value: f32) -> f32 {
	if value == 0.0 {
		0.0
	} else {
		value
	}
}

/// Finite parameter bits after signed-zero normalization. `None` if non-finite.
pub fn finite_parameter_bits(value: f32) -> Option<u32> {
	let value = normalize_signed_zero(value);
	value.is_finite().then_some(value.to_bits())
}

fn cycle_address(clip_time: f32, interval: f32, duration: f32) -> SampleAddress {
	let duration = duration.max(interval);
	if !clip_time.is_finite() {
		return SampleAddress { index: 0, canonical_time: 0.0 };
	}
	let wrapped = clip_time.rem_euclid(duration);
	let bins = bins_in_span(duration, interval);
	let mut index = (wrapped / interval).round();
	if index >= bins as f32 || index < 0.0 {
		index = 0.0;
	}
	let index = index as u32;
	SampleAddress { index, canonical_time: index as f32 * interval }
}

fn clamp_address(clip_time: f32, interval: f32, duration: f32) -> SampleAddress {
	let duration = duration.max(0.0);
	if !clip_time.is_finite() || clip_time <= 0.0 || duration == 0.0 {
		return SampleAddress { index: 0, canonical_time: 0.0 };
	}
	if clip_time >= duration {
		return SampleAddress { index: bins_in_span(duration, interval), canonical_time: duration };
	}
	let max_index = bins_in_span(duration, interval);
	let index = (clip_time / interval).round().clamp(0.0, max_index as f32) as u32;
	if index == max_index {
		SampleAddress { index, canonical_time: duration }
	} else {
		SampleAddress { index, canonical_time: index as f32 * interval }
	}
}

fn unbounded_address(clip_time: f32, interval: f32) -> SampleAddress {
	if !clip_time.is_finite() || clip_time <= 0.0 {
		return SampleAddress { index: 0, canonical_time: 0.0 };
	}
	let index_f = (clip_time / interval).round().max(0.0);
	let index = if index_f >= u32::MAX as f32 { u32::MAX } else { index_f as u32 };
	SampleAddress { index, canonical_time: index as f32 * interval }
}

fn bins_in_span(duration: f32, interval: f32) -> u32 {
	let bins = (duration / interval).round().max(1.0);
	if bins >= u32::MAX as f32 {
		u32::MAX
	} else {
		bins as u32
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const TEN_MS: u32 = DEFAULT_SAMPLE_INTERVAL_US;

	#[test]
	fn default_interval_is_ten_milliseconds() {
		assert!((interval_seconds(TEN_MS) - 0.01).abs() < 1e-7);
		assert!((interval_seconds(0) - 0.01).abs() < 1e-7);
	}

	#[test]
	fn cycle_selects_nearest_bin() -> anyhow::Result<()> {
		let policy = ClipTimePolicy::Cycle { duration: 1.0 };
		let mid = SampleAddress::from_clip_time(0.121, TEN_MS, policy);
		if mid.index != 12 || (mid.canonical_time - 0.12).abs() > 1e-6 {
			anyhow::bail!("expected bin 12 @ 0.12, got {mid:?}");
		}
		let other = SampleAddress::from_clip_time(0.123, TEN_MS, policy);
		if other.index != mid.index || (other.canonical_time - mid.canonical_time).abs() > 1e-6 {
			anyhow::bail!("times in the same bin must share the canonical sample");
		}
		Ok(())
	}

	#[test]
	fn cycle_wraps_the_loop_seam() -> anyhow::Result<()> {
		let policy = ClipTimePolicy::Cycle { duration: 1.0 };
		let before = SampleAddress::from_clip_time(0.996, TEN_MS, policy);
		let after = SampleAddress::from_clip_time(1.004, TEN_MS, policy);
		if before.index != 0 || after.index != 0 {
			anyhow::bail!("loop seam should wrap to bin 0, got {before:?} / {after:?}");
		}
		if before.canonical_time != 0.0 || after.canonical_time != 0.0 {
			anyhow::bail!("wrapped canonical time must be 0");
		}
		let interior = SampleAddress::from_clip_time(0.994, TEN_MS, policy);
		if interior.index != 99 || (interior.canonical_time - 0.99).abs() > 1e-6 {
			anyhow::bail!("0.994 should stay on bin 99, got {interior:?}");
		}
		Ok(())
	}

	#[test]
	fn cycle_preserves_the_half_cycle_discontinuity() -> anyhow::Result<()> {
		let policy = ClipTimePolicy::Cycle { duration: 1.0 };
		let at_half = SampleAddress::from_clip_time(0.5, TEN_MS, policy);
		if at_half.index != 50 || (at_half.canonical_time - 0.5).abs() > 1e-6 {
			anyhow::bail!("walk/run peak at 0.5 must land on an exact bin, got {at_half:?}");
		}
		Ok(())
	}

	#[test]
	fn clamp_keeps_exact_endpoints() -> anyhow::Result<()> {
		let policy = ClipTimePolicy::Clamp { duration: 1.0 };
		let start = SampleAddress::from_clip_time(-0.2, TEN_MS, policy);
		let end = SampleAddress::from_clip_time(1.4, TEN_MS, policy);
		let exact_end = SampleAddress::from_clip_time(1.0, TEN_MS, policy);
		if start.index != 0 || start.canonical_time != 0.0 {
			anyhow::bail!("start endpoint must be 0, got {start:?}");
		}
		if end.canonical_time != 1.0 || exact_end.canonical_time != 1.0 {
			anyhow::bail!("end endpoint must stay 1.0, got {end:?} / {exact_end:?}");
		}
		if end.index != exact_end.index {
			anyhow::bail!("clamped past-end and exact end must share an index");
		}
		Ok(())
	}

	#[test]
	fn unbounded_marches_with_clip_time() -> anyhow::Result<()> {
		let policy = ClipTimePolicy::Unbounded;
		let first = SampleAddress::from_clip_time(3.202, TEN_MS, policy);
		let second = SampleAddress::from_clip_time(3.204, TEN_MS, policy);
		if first.index != 320 || (first.canonical_time - 3.20).abs() > 1e-5 {
			anyhow::bail!("expected idle-style bin 320 @ 3.20, got {first:?}");
		}
		if first.index != second.index {
			anyhow::bail!("nearby unbounded times must share a bin");
		}
		Ok(())
	}

	#[test]
	fn signed_zero_normalizes() {
		assert_eq!(normalize_signed_zero(-0.0).to_bits(), 0.0f32.to_bits());
		assert_eq!(finite_parameter_bits(-0.0), finite_parameter_bits(0.0));
		assert!(finite_parameter_bits(f32::NAN).is_none());
		assert!(finite_parameter_bits(f32::INFINITY).is_none());
	}

	#[test]
	fn table_len_matches_addressable_bins() -> anyhow::Result<()> {
		if (ClipTimePolicy::Cycle { duration: 1.0 }).table_len(TEN_MS) != Some(100) {
			anyhow::bail!("a 1s walk/run cycle is 100 bins at 10 ms");
		}
		if (ClipTimePolicy::Clamp { duration: 1.0 }).table_len(TEN_MS) != Some(101) {
			anyhow::bail!("clamp keeps both endpoints, so 101 slots");
		}
		if ClipTimePolicy::Unbounded.table_len(TEN_MS).is_some() {
			anyhow::bail!("unbounded idle has no finite table");
		}
		Ok(())
	}
}
