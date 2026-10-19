//! Shared timing, stats, and A/B reporting.

use std::time::{Duration, Instant};

use anyhow::Result;

pub const DEFAULT_WARMUP: usize = 1;
pub const DEFAULT_REPEATS: usize = 7;

/// How a destination lives across measured passes.
///
/// Callers apply this themselves. Putting it on the report keeps the lifetime
/// in the output instead of burying it in a helper that clears or allocates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassLifetime {
	/// Clear in place each pass; keep allocated capacity (cull cache).
	ClearedPerPass,
	/// Allocate a new destination each pass (produce `HashSet` per domain).
	FreshPerPass,
	/// Keep contents across passes. Not a production cull/produce lifetime.
	Retained,
}

impl PassLifetime {
	pub fn label(self) -> &'static str {
		match self {
			Self::ClearedPerPass => "cleared-per-pass (capacity retained)",
			Self::FreshPerPass => "fresh-per-pass",
			Self::Retained => "retained across passes",
		}
	}
}

/// Warmup + timed repeats. One sample is whatever the caller closure does.
#[derive(Clone, Copy, Debug)]
pub struct Microbench {
	pub warmup: usize,
	pub repeats: usize,
}

impl Default for Microbench {
	fn default() -> Self {
		Self { warmup: DEFAULT_WARMUP, repeats: DEFAULT_REPEATS }
	}
}

impl Microbench {
	pub fn collect<F>(&self, mut run: F) -> Result<Vec<Duration>>
	where
		F: FnMut() -> Result<()>,
	{
		for _ in 0..self.warmup {
			run()?;
		}
		let mut samples = Vec::with_capacity(self.repeats);
		for _ in 0..self.repeats {
			samples.push(Self::time(&mut run)?);
		}
		Ok(samples)
	}

	/// Time A and B in alternating order so sample `i` does not always run A first.
	pub fn collect_ab_alternating<A, B>(
		&self,
		mut original: A,
		mut revised: B,
	) -> Result<(Vec<Duration>, Vec<Duration>)>
	where
		A: FnMut() -> Result<()>,
		B: FnMut() -> Result<()>,
	{
		for _ in 0..self.warmup {
			original()?;
			revised()?;
		}
		let mut original_samples = Vec::with_capacity(self.repeats);
		let mut revised_samples = Vec::with_capacity(self.repeats);
		for i in 0..self.repeats {
			if i % 2 == 0 {
				original_samples.push(Self::time(&mut original)?);
				revised_samples.push(Self::time(&mut revised)?);
			} else {
				revised_samples.push(Self::time(&mut revised)?);
				original_samples.push(Self::time(&mut original)?);
			}
		}
		Ok((original_samples, revised_samples))
	}

	fn time<F: FnMut() -> Result<()>>(run: &mut F) -> Result<Duration> {
		let started = Instant::now();
		run()?;
		Ok(started.elapsed())
	}
}

/// min / median / mean / max over timed samples.
#[derive(Clone, Copy, Debug)]
pub struct DurationStats {
	pub min: Duration,
	pub median: Duration,
	pub mean: Duration,
	pub max: Duration,
}

impl DurationStats {
	pub fn from_samples(samples: &[Duration]) -> Result<Self> {
		if samples.is_empty() {
			return Err(anyhow::anyhow!("microbench produced no samples"));
		}
		let mut sorted = samples.to_vec();
		sorted.sort();
		Ok(Self {
			min: sorted[0],
			median: sorted[sorted.len() / 2],
			mean: samples.iter().copied().sum::<Duration>() / u32::try_from(samples.len())?,
			max: sorted[sorted.len() - 1],
		})
	}

	pub fn per_op(self, operations: u32) -> Result<Self> {
		if operations == 0 {
			return Err(anyhow::anyhow!("cannot normalize by zero operations"));
		}
		Ok(Self {
			min: self.min / operations,
			median: self.median / operations,
			mean: self.mean / operations,
			max: self.max / operations,
		})
	}

	pub fn format(self) -> String {
		format!("{:?} / {:?} / {:?} / {:?}", self.min, self.median, self.mean, self.max)
	}
}

/// A/B reporter: window totals, per-operation times, and mean ratio.
pub struct AbReport<'a> {
	pub label: &'a str,
	pub original_name: &'a str,
	pub revised_name: &'a str,
	pub original: &'a [Duration],
	pub revised: &'a [Duration],
	pub operations: u32,
	pub operation_name: &'a str,
	pub lifetime: PassLifetime,
	pub notes: &'a str,
}

impl AbReport<'_> {
	pub fn write_to_stderr(&self) -> Result<()> {
		let original = DurationStats::from_samples(self.original)?;
		let revised = DurationStats::from_samples(self.revised)?;
		let original_per = original.per_op(self.operations)?;
		let revised_per = revised.per_op(self.operations)?;
		let ratio = if revised.mean.is_zero() {
			f64::INFINITY
		} else {
			original.mean.as_secs_f64() / revised.mean.as_secs_f64()
		};
		eprintln!(
			"{label}: {notes}; lifetime {lifetime}; \
			 {original_name} min/med/mean/max {original}; \
			 {revised_name} {revised}; \
			 per-{operation} original {original_per} revised {revised_per}; \
			 mean ratio {ratio:.2}×",
			label = self.label,
			notes = self.notes,
			lifetime = self.lifetime.label(),
			original_name = self.original_name,
			original = original.format(),
			revised_name = self.revised_name,
			revised = revised.format(),
			operation = self.operation_name,
			original_per = original_per.format(),
			revised_per = revised_per.format(),
			ratio = ratio,
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stats_from_samples() -> Result<()> {
		let samples =
			[Duration::from_millis(4), Duration::from_millis(2), Duration::from_millis(6)];
		let stats = DurationStats::from_samples(&samples)?;
		assert_eq!(stats.min, Duration::from_millis(2));
		assert_eq!(stats.median, Duration::from_millis(4));
		assert_eq!(stats.max, Duration::from_millis(6));
		assert_eq!(stats.mean, Duration::from_millis(4));
		assert_eq!(stats.per_op(2)?.mean, Duration::from_millis(2));
		Ok(())
	}

	#[test]
	fn alternating_order_runs_both_sides() -> Result<()> {
		let mut original_n = 0usize;
		let mut revised_n = 0usize;
		let (original, revised) = Microbench { warmup: 1, repeats: 4 }.collect_ab_alternating(
			|| {
				original_n += 1;
				Ok(())
			},
			|| {
				revised_n += 1;
				Ok(())
			},
		)?;
		assert_eq!(original.len(), 4);
		assert_eq!(revised.len(), 4);
		assert_eq!(original_n, 5);
		assert_eq!(revised_n, 5);
		Ok(())
	}
}
