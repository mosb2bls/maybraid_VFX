//! Continuous roof geometry primitives.

/// Continuous roof / cap forms. Tessellation into kit pieces is private.
#[derive(Debug, Clone, PartialEq)]
pub enum RoofGeometry {
	/// Pitched trapezoid: rectangular body plus optional end triangles.
	Pitch(Pitch),
	/// Dome sweep filled with 180° / 90° / 15° arc kits (empty leaves for now).
	Dome(DomeRoof),
}

impl Default for RoofGeometry {
	fn default() -> Self {
		Self::Dome(DomeRoof::default())
	}
}

impl RoofGeometry {
	pub fn pitch(pitch: Pitch) -> Self {
		Self::Pitch(pitch)
	}

	pub fn dome(sweep_degrees: f32) -> Self {
		Self::Dome(DomeRoof { sweep_degrees })
	}

	/// Pitch about local +X in radians (`atan2(rise, run)`). Domes are unpitched at
	/// the geometry root.
	pub fn pitch_radians(&self) -> f32 {
		match self {
			Self::Pitch(p) => p.pitch_radians(),
			Self::Dome(_) => 0.0,
		}
	}

	/// Pitch about local +X in degrees. Domes are unpitched (`0`).
	pub fn pitch_degrees(&self) -> f32 {
		f32::to_degrees(self.pitch_radians())
	}
}

/// Alias kept for migration; prefer [`RoofGeometry`].
pub type Roof = RoofGeometry;

/// Pitched roof face: rectangle along the eave/ridge plus optional end triangles.
///
/// Pitch-space axes: **X** along eave/ridge length, **Z** run (eave at \(Z = 0\),
/// ridge at \(Z = -\texttt{run}\)), **Y** rise via rotation about +X.
///
/// Anchor is the **lower-left** of the full extent (left end triangle if present,
/// otherwise the rectangle's left edge; eave at \(Y = 0\)). `rise` / `run` must be
/// non-negative; invert orientation via placement rotation instead.
///
/// Roof-native authoring — tessellates directly to rectangle / right-triangle kits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pitch {
	/// How much the pitch rises from eave to roofline (pitch-space Y).
	pub rise: f32,
	/// How far from eave to midline / ridge (pitch-space Z extent).
	pub run: f32,
	/// Rectangular left-to-right span. `None` omits the rectangular body.
	pub length: Option<f32>,
	/// Suggested tile width along X; fitted so \(n\) tiles span `length` exactly.
	pub tile_width: f32,
	/// Optional left end-triangle base length (absolute). Sign: positive = upright
	/// (eave-long), negative = flipped (ridge-long). `None` omits the triangle.
	pub left: Option<f32>,
	/// Optional right end-triangle base length (absolute). Same sign convention as
	/// [`Self::left`].
	pub right: Option<f32>,
}

impl Pitch {
	pub fn new(rise: f32, run: f32, tile_width: f32) -> Self {
		Self {
			rise: rise.max(0.0),
			run: run.max(0.0),
			length: None,
			tile_width: tile_width.max(1e-4),
			left: None,
			right: None,
		}
	}

	pub fn with_length(mut self, length: f32) -> Self {
		self.length = Some(length.max(0.0));
		self
	}

	pub fn with_left(mut self, base: f32) -> Self {
		self.left = Some(base);
		self
	}

	pub fn with_right(mut self, base: f32) -> Self {
		self.right = Some(base);
		self
	}

	/// Set left end base from a plan-view angle (degrees): \(\texttt{base} = \texttt{run}
	/// \cdot \tan\theta\). Sign of `angle_degrees` becomes the triangle flip sign.
	pub fn with_left_angle(mut self, angle_degrees: f32) -> Self {
		self.left = Some(self.run * f32::to_radians(angle_degrees).tan());
		self
	}

	/// Set right end base from a plan-view angle (degrees). See [`Self::with_left_angle`].
	pub fn with_right_angle(mut self, angle_degrees: f32) -> Self {
		self.right = Some(self.run * f32::to_radians(angle_degrees).tan());
		self
	}

	/// Build a pitch whose rectangular span is \(\min(\texttt{eave}, \texttt{ridge})\) and
	/// whose end triangles equally absorb \(|\texttt{ridge} - \texttt{eave}|\).
	///
	/// When the ridge is longer, both ends are flipped (negative base); when the eave
	/// is longer, both ends are upright (positive base).
	pub fn from_eave_ridge(rise: f32, run: f32, eave: f32, ridge: f32, tile_width: f32) -> Self {
		let rise = rise.max(0.0);
		let run = run.max(0.0);
		let eave = eave.max(0.0);
		let ridge = ridge.max(0.0);
		let rect = eave.min(ridge);
		let half_diff = (ridge - eave).abs() * 0.5;
		let end = if half_diff <= 1e-6 {
			None
		} else if ridge > eave {
			Some(-half_diff)
		} else {
			Some(half_diff)
		};
		Self {
			rise,
			run,
			length: if rect > 1e-6 { Some(rect) } else { None },
			tile_width: tile_width.max(1e-4),
			left: end,
			right: end,
		}
	}

	pub fn pitch_radians(self) -> f32 {
		let run = self.run.max(1e-4);
		f32::atan2(self.rise.max(0.0), run)
	}

	/// Full X extent including optional end triangles (lower-left anchored at 0).
	pub fn extent_x(self) -> f32 {
		self.left.map(|b| b.abs()).unwrap_or(0.0)
			+ self.length.unwrap_or(0.0)
			+ self.right.map(|b| b.abs()).unwrap_or(0.0)
	}

	/// X offset where the rectangular body starts (after left end triangle).
	pub fn rect_origin_x(self) -> f32 {
		self.left.map(|b| b.abs()).unwrap_or(0.0)
	}
}

/// Dome roof filled via the shared 180° / 90° / 15° arc kit standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeRoof {
	pub sweep_degrees: f32,
}

impl Default for DomeRoof {
	fn default() -> Self {
		Self { sweep_degrees: 360.0 }
	}
}
