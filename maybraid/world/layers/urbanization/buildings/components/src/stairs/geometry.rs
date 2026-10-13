//! Continuous stair geometry.

#[derive(Debug, Clone, PartialEq)]
pub enum StairGeometry {
	Spiral(SpiralStair),
	Straight(StraightStair),
}

impl Default for StairGeometry {
	fn default() -> Self {
		Self::straight()
	}
}

impl StairGeometry {
	pub fn spiral() -> Self {
		Self::Spiral(SpiralStair::default())
	}

	/// Circular run of treads that rises `height` meters around `radius`.
	pub fn spiral_run(height: f32, radius: f32, tread_width: f32, tread_depth: f32) -> Self {
		Self::Spiral(SpiralStair {
			height: height.max(1e-4),
			radius: radius.max(1e-4),
			tread_width: tread_width.max(1e-4),
			tread_depth: tread_depth.max(1e-4),
			tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
			turns: 1.0,
			tread_tops: Vec::new(),
		})
	}

	/// Circular run with explicit local tread-top \(Y\) bindings (ascending).
	pub fn spiral_fitted(
		radius: f32,
		tread_width: f32,
		tread_depth: f32,
		tread_tops: Vec<f32>,
		turns: f32,
	) -> Self {
		let tops = normalize_tops(tread_tops);
		let height = tops.last().copied().unwrap_or(SpiralStair::DEFAULT_TREAD_HEIGHT);
		Self::Spiral(SpiralStair {
			height: height.max(1e-4),
			radius: radius.max(1e-4),
			tread_width: tread_width.max(1e-4),
			tread_depth: tread_depth.max(1e-4),
			tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
			turns: turns.max(1e-4),
			tread_tops: tops,
		})
	}

	pub fn straight() -> Self {
		Self::Straight(StraightStair::default())
	}

	/// Linear run of treads along local \(+X\).
	pub fn straight_run(height: f32, length: f32, width: f32, depth: f32) -> Self {
		Self::Straight(StraightStair::run(height, length, width, depth))
	}
}

/// Alias kept for migration; prefer [`StairGeometry`].
pub type Stair = StairGeometry;

/// Spiral / circular stair parameterized for tread tessellation.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralStair {
	/// Total rise to the next level (meters). Ignored when [`Self::tread_tops`] is non-empty
	/// except as a fallback height.
	pub height: f32,
	/// Centerline radius of the tread run.
	pub radius: f32,
	/// Radial tread width (world meters).
	pub tread_width: f32,
	/// Tangential tread depth / run (world meters).
	pub tread_depth: f32,
	/// Target rise per tread (~0.18 m per README) for uniform runs.
	pub tread_height: f32,
	/// Number of full turns over the run (1.0 = one revolution).
	pub turns: f32,
	/// Local tread-top \(Y\) values (ascending). Empty → uniform rise from
	/// [`Self::height`] / [`Self::tread_height`].
	pub tread_tops: Vec<f32>,
}

impl SpiralStair {
	pub const DEFAULT_TREAD_HEIGHT: f32 = 0.18;

	pub fn tread_count(&self) -> u32 {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.len() as u32;
		}
		let h = self.tread_height.max(1e-4);
		(self.height / h).ceil().max(1.0) as u32
	}

	/// Exact rise per tread so a uniform run lands on `height`.
	pub fn rise_per_tread(&self) -> f32 {
		self.height / self.tread_count() as f32
	}

	/// Resolved ascending local tread-top \(Y\)s.
	pub fn effective_tread_tops(&self) -> Vec<f32> {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.clone();
		}
		let n = self.tread_count();
		let rise = self.rise_per_tread();
		(1..=n).map(|i| i as f32 * rise).collect()
	}
}

impl Default for SpiralStair {
	fn default() -> Self {
		Self {
			height: 3.0,
			radius: 1.0,
			tread_width: 0.5,
			tread_depth: 0.35,
			tread_height: Self::DEFAULT_TREAD_HEIGHT,
			turns: 1.0,
			tread_tops: Vec::new(),
		}
	}
}

/// Linear stair: treads step along local \(+X\) (travel), width along local \(Z\).
///
/// `length` is the total going. `width` / `depth` are one tread's across / going.
/// Empty [`Self::tread_tops`] ⇒ uniform rise from [`Self::height`] / [`Self::tread_height`].
/// Placement is the **walkable** center of the first tread (\(X \in [-1, 1]\)).
/// Kits may bleed to \(X = -2\); [`Self::flush_start`] packs that into the first going.
#[derive(Debug, Clone, PartialEq)]
pub struct StraightStair {
	/// Total rise (meters). Ignored when [`Self::tread_tops`] is non-empty except as
	/// a fallback height.
	pub height: f32,
	/// Total horizontal going (meters).
	pub length: f32,
	/// Tread width, across the run (meters).
	pub width: f32,
	/// One tread's going / depth (meters).
	pub depth: f32,
	/// Target rise per tread (~0.18 m) for uniform runs.
	pub tread_height: f32,
	/// Local tread-top \(Y\) values (ascending). Empty → uniform rise.
	pub tread_tops: Vec<f32>,
	/// Pack the first kit's rearward bleed (\(X \to -2\)) into the going.
	///
	/// Walkable contact stays \(X \in [-1, 1]\). Leave `false` on one-tread
	/// circular nodes so adjacent kits still nest on the bleed.
	pub flush_start: bool,
}

impl StraightStair {
	pub const DEFAULT_TREAD_HEIGHT: f32 = 0.18;
	pub const DEFAULT_WIDTH: f32 = 0.5;
	pub const DEFAULT_DEPTH: f32 = 0.35;

	/// Uniform run that fills `length` with treads of going `depth`.
	pub fn run(height: f32, length: f32, width: f32, depth: f32) -> Self {
		let height = height.max(1e-4);
		let depth = depth.max(1e-4);
		let length = length.max(depth);
		Self {
			height,
			length,
			width: width.max(1e-4),
			depth,
			tread_height: Self::DEFAULT_TREAD_HEIGHT,
			tread_tops: Vec::new(),
			flush_start: false,
		}
	}

	/// One tread. Placement is the tread's plan center at the walk-on \(Y\).
	pub fn single(width: f32, depth: f32, rise: f32) -> Self {
		let depth = depth.max(1e-4);
		Self::run(rise.max(1e-4), depth, width, depth)
	}

	/// First kit does not hang the \(X = -2\) bleed behind the walkable trailing.
	pub fn with_flush_start(mut self, flush: bool) -> Self {
		self.flush_start = flush;
		self
	}

	/// Replace uniform rise with explicit local tread-top \(Y\)s.
	pub fn with_tread_tops(mut self, tops: Vec<f32>) -> Self {
		self.tread_tops = tops;
		self
	}

	pub fn tread_count(&self) -> u32 {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.len() as u32;
		}
		(self.height / self.tread_height.max(1e-4)).ceil().max(1.0) as u32
	}

	/// Exact rise per tread so a uniform run lands on `height`.
	pub fn rise_per_tread(&self) -> f32 {
		self.height / self.tread_count() as f32
	}

	/// Going per tread so a uniform run fills `length`.
	pub fn going_per_tread(&self) -> f32 {
		self.length / self.tread_count() as f32
	}

	/// Resolved ascending local tread-top \(Y\)s.
	pub fn effective_tread_tops(&self) -> Vec<f32> {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.clone();
		}
		let n = self.tread_count();
		let rise = self.rise_per_tread();
		(1..=n).map(|i| i as f32 * rise).collect()
	}
}

impl Default for StraightStair {
	fn default() -> Self {
		Self::run(3.0, 3.0 * Self::DEFAULT_DEPTH, Self::DEFAULT_WIDTH, Self::DEFAULT_DEPTH)
	}
}

fn normalize_tops(mut tops: Vec<f32>) -> Vec<f32> {
	tops.retain(|y| y.is_finite());
	tops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	tops.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
	tops
}
