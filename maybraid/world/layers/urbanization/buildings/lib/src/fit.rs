//! Shared fitting contract: place a type inside [`Confines`] from spatial noise.
//!
//! Most complete fitting types follow **Parameterized → FloorPlan → Full\***:
//! sample knobs from noise, emit structure + residual [`FillableRegions`], then
//! fill those regions (or leave them for a later typology).
//!
//! **Towering:** sample a FloorPlan once, then rebuild upper levels with
//! `from_parameterized` + Y-lifted confines so the plan stays identical. When
//! stacking the *same* storey type, prefer that path and ignore [`FillableRegions::atop`]
//! so irregular footprints are not re-derived per level.
//!
//! Axis-aligned bound helpers used while fitting live here alongside [`Confines`]
//! ([`aabb_near_plane`], [`aabb_xz_near_eq`], [`aabb_xz_overlap_area`], …).

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;

use crate::openings::Openings;

/// Describes the confines in which the object must fit.
#[derive(Debug, Clone, PartialEq)]
pub struct Confines {
	/// The bounding box of the confines.
	pub bounds: Aabb3d,
	/// The roll of the confines.
	///
	/// Typically consumed by higher-order types to rotate all primitives emitted.
	pub roll: f32,
	/// Openings required on (or reserved within) the confines.
	pub openings: Openings,
}

impl Confines {
	pub fn new(bounds: Aabb3d, roll: f32, openings: Openings) -> Self {
		Self { bounds, roll, openings }
	}

	pub fn from_bounds(bounds: Aabb3d) -> Self {
		Self::new(bounds, 0.0, Openings::new())
	}

	/// World-space center of [`Self::bounds`] (spatial noise sample point).
	pub fn center(&self) -> Vec3 {
		self.bounds.center().into()
	}

	/// Plan-space center \((x, z)\) of [`Self::bounds`].
	pub fn center_xz(&self) -> Vec2 {
		aabb_xz_center(&self.bounds)
	}

	/// Footprint extents \((x, z)\) of [`Self::bounds`].
	pub fn footprint(&self) -> Vec2 {
		aabb_xz_extent(&self.bounds)
	}
}

/// True when the closed interval `[lo, hi]` reaches within `tol` of `plane`.
#[inline]
pub fn aabb_near_plane(lo: f32, hi: f32, plane: f32, tol: f32) -> bool {
	lo <= plane + tol && hi >= plane - tol
}

/// Plan-space center \((x, z)\) of an AABB.
#[inline]
pub fn aabb_xz_center(a: &Aabb3d) -> Vec2 {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	Vec2::new((min.x + max.x) * 0.5, (min.z + max.z) * 0.5)
}

/// Plan footprint size \((x, z)\) of an AABB.
#[inline]
pub fn aabb_xz_extent(a: &Aabb3d) -> Vec2 {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0))
}

/// Plan footprint area \(x \cdot z\) of an AABB.
#[inline]
pub fn aabb_xz_area(a: &Aabb3d) -> f32 {
	let e = aabb_xz_extent(a);
	e.x * e.y
}

/// True when two AABBs share nearly the same XZ footprint (Y ignored).
#[inline]
pub fn aabb_xz_near_eq(a: &Aabb3d, b: &Aabb3d, eps: f32) -> bool {
	let amin = Vec3::from(a.min);
	let amax = Vec3::from(a.max);
	let bmin = Vec3::from(b.min);
	let bmax = Vec3::from(b.max);
	(amin.x - bmin.x).abs() < eps
		&& (amin.z - bmin.z).abs() < eps
		&& (amax.x - bmax.x).abs() < eps
		&& (amax.z - bmax.z).abs() < eps
}

/// Overlap area between an AABB’s XZ footprint and a plan [`Aabb2d`]
/// (\(x → X\), \(y → Z\)).
#[inline]
pub fn aabb_xz_overlap_area(a: &Aabb3d, region: &Aabb2d) -> f32 {
	let amin = Vec3::from(a.min);
	let amax = Vec3::from(a.max);
	let x0 = amin.x.max(region.min.x);
	let x1 = amax.x.min(region.max.x);
	let z0 = amin.z.max(region.min.y);
	let z1 = amax.z.min(region.max.y);
	(x1 - x0).max(0.0) * (z1 - z0).max(0.0)
}

/// Vertical box atop the confines on which other types can stack.
///
/// [`Self::bounds`] is the plan footprint: `Aabb2d` \(x → world X, y → world Z\).
#[derive(Debug, Clone, PartialEq)]
pub struct StackRegion {
	/// Plan footprint (\(x, z\) as [`Aabb2d`] \(x, y\)).
	pub bounds: Aabb2d,
	/// Height of the stack region (world Y extent available above the storey).
	pub height: f32,
	/// Roll of the stack region.
	pub roll: f32,
	/// Openings required when stacking on this region.
	pub openings: Openings,
}

/// Semantic role of a residual [`FillRegion`] (helps Full\* composition).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpaceKind {
	/// Enclosed volume (rooms, shafts, cores).
	InternalSpace,
	/// Street- or courtyard-facing commercial / facade band.
	ExternalSpace,
	/// Open walking band (balconies, galleries without stalls).
	Walkway,
	/// Linear circulation between spaces.
	Hallway,
	/// Storage / leftover pocket (not primary living program).
	ClosetSpace,
	/// Author-defined label when none of the above fit.
	Custom(String),
}

/// One residual fill slot: typed [`Confines`].
#[derive(Debug, Clone, PartialEq)]
pub struct FillRegion {
	pub kind: SpaceKind,
	pub confines: Confines,
}

impl FillRegion {
	pub fn new(kind: SpaceKind, confines: Confines) -> Self {
		Self { kind, confines }
	}
}

/// Residual regions after a successful fit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillableRegions {
	/// Regions that can be filled within.
	///
	/// Used for composing so livable types need not fully describe internal
	/// confines (e.g. FloorPlan → FullStorey). The FloorPlan emits structure and
	/// `within`; Full\* fills those confines with specific types. The same
	/// FloorPlan can back other Full\* variants.
	pub within: Vec<FillRegion>,
	/// Regions on which we can stack.
	///
	/// Typically used when transitioning between tower and storey types. When
	/// using the same storey, prefer towering via a copied floor plan and ignore
	/// this so irregular geometry stays stable.
	pub atop: Vec<StackRegion>,
}

impl FillableRegions {
	pub fn empty() -> Self {
		Self::default()
	}

	/// Append another residual set (`within` then `atop`).
	pub fn extend(&mut self, other: FillableRegions) {
		self.within.extend(other.within);
		self.atop.extend(other.atop);
	}
}

/// Several typed rectangular pieces (L / T / grouped cells) offered for filling.
///
/// Each part is a [`FillRegion`] so [`SpaceKind`] and openings stay with the
/// piece. Leaf Fits that only implement [`Fit::fit_to_confines`] still work via
/// the default [`Fit::fit_to_multi_confines`] map.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MultiConfines {
	pub parts: Vec<FillRegion>,
}

impl MultiConfines {
	pub fn new(parts: impl IntoIterator<Item = FillRegion>) -> Self {
		Self { parts: parts.into_iter().collect() }
	}

	pub fn empty() -> Self {
		Self::default()
	}

	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	pub fn len(&self) -> usize {
		self.parts.len()
	}

	pub fn iter(&self) -> impl Iterator<Item = &FillRegion> {
		self.parts.iter()
	}

	/// Build from untyped confines (all [`SpaceKind::InternalSpace`]).
	pub fn from_confines(parts: impl IntoIterator<Item = Confines>) -> Self {
		Self::new(parts.into_iter().map(|c| FillRegion::new(SpaceKind::InternalSpace, c)))
	}
}

impl From<Confines> for MultiConfines {
	fn from(confines: Confines) -> Self {
		Self::from_confines([confines])
	}
}

impl From<FillRegion> for MultiConfines {
	fn from(region: FillRegion) -> Self {
		Self::new([region])
	}
}

impl From<Vec<FillRegion>> for MultiConfines {
	fn from(parts: Vec<FillRegion>) -> Self {
		Self { parts }
	}
}

/// Result of fitting into [`MultiConfines`] (or a single [`Confines`] via [`Fit::fit_to`]).
///
/// Soft piece rejects (`TooSmall`) stay in [`Self::residual`] so a higher-order
/// chooser can try another typology. Nested residuals from successful pieces are
/// merged into [`Self::residual`] as well.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiFit<T> {
	/// Successfully fitted pieces (order matches successful inputs).
	pub fitted: Vec<(T, FillableRegions)>,
	/// Unfilled input pieces plus nested residuals from successes.
	pub residual: FillableRegions,
}

impl<T> MultiFit<T> {
	pub fn empty() -> Self {
		Self { fitted: Vec::new(), residual: FillableRegions::empty() }
	}

	pub fn is_empty(&self) -> bool {
		self.fitted.is_empty()
	}

	/// Number of successfully fitted pieces.
	pub fn fitted_len(&self) -> usize {
		self.fitted.len()
	}
}

/// Single rectangle or multi-rectangle fit target.
#[derive(Debug, Clone, PartialEq)]
pub enum FitTarget {
	Single(Confines),
	Multi(MultiConfines),
}

impl From<Confines> for FitTarget {
	fn from(confines: Confines) -> Self {
		Self::Single(confines)
	}
}

impl From<MultiConfines> for FitTarget {
	fn from(multi: MultiConfines) -> Self {
		Self::Multi(multi)
	}
}

/// Failure modes for [`Fit::fit_to_confines`].
///
/// Soft volume rejects use [`FitError::TooSmall`] so a higher-order chooser can
/// try another typology. Hard invariant breaks use [`FitError::InvalidConfines`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
	/// Confines are too small for this type's minimum extents.
	TooSmall {
		/// Short reason (e.g. `"footprint"`, `"height"`).
		reason: &'static str,
	},
	/// Confines are malformed or otherwise unusable.
	InvalidConfines { reason: &'static str },
}

impl std::fmt::Display for FitError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TooSmall { reason } => write!(f, "too small: {reason}"),
			Self::InvalidConfines { reason } => write!(f, "invalid confines: {reason}"),
		}
	}
}

impl std::error::Error for FitError {}

/// Fit a type into confines from spatial noise.
///
/// Implementors should sample with [`NoiseParams`] → [`procedural_common::NoiseConfig`]
/// at the confines center (distinct `w` salt lanes per decision). Soft rejects
/// return [`FitError::TooSmall`].
///
/// Only [`Self::fit_to_confines`] is required. [`Self::fit_to_multi_confines`]
/// defaults to mapping over each [`MultiConfines`] part (partial fill: keep
/// successes, residualize `TooSmall`). Types that need joint multi-cell layout
/// can override it.
pub trait Fit: Sized {
	/// Fit `Self` to `confines` using spatial `noise`.
	///
	/// On success, returns the fitted object and residual fillable regions.
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError>;

	/// Fit into several typed rectangles (L / grouped cells).
	///
	/// **Default:** call [`Self::fit_to_confines`] per part.
	/// - `Ok` → push onto [`MultiFit::fitted`]; merge nested residuals into
	///   [`MultiFit::residual`].
	/// - `Err(TooSmall)` → push the original [`FillRegion`] into residual
	///   (partial fill).
	/// - `Err(InvalidConfines)` → fail the whole multi fit.
	///
	/// An all-`TooSmall` input still returns `Ok` with empty `fitted` so a
	/// chooser can try another typology on the residuals.
	fn fit_to_multi_confines(
		multi: &MultiConfines,
		noise: NoiseParams,
	) -> Result<MultiFit<Self>, FitError> {
		let mut fitted = Vec::new();
		let mut residual = FillableRegions::empty();
		for part in multi.iter() {
			match Self::fit_to_confines(&part.confines, noise) {
				Ok((value, nested)) => {
					residual.extend(nested.clone());
					fitted.push((value, nested));
				}
				Err(FitError::TooSmall { .. }) => {
					residual.within.push(part.clone());
				}
				Err(err) => return Err(err),
			}
		}
		Ok(MultiFit { fitted, residual })
	}

	/// Dispatch on [`FitTarget`].
	fn fit_to(target: &FitTarget, noise: NoiseParams) -> Result<MultiFit<Self>, FitError> {
		match target {
			FitTarget::Single(confines) => match Self::fit_to_confines(confines, noise) {
				Ok((value, nested)) => {
					Ok(MultiFit { fitted: vec![(value, nested.clone())], residual: nested })
				}
				Err(FitError::TooSmall { .. }) => Ok(MultiFit {
					fitted: Vec::new(),
					residual: FillableRegions {
						within: vec![FillRegion::new(SpaceKind::InternalSpace, confines.clone())],
						atop: Vec::new(),
					},
				}),
				Err(err) => Err(err),
			},
			FitTarget::Multi(multi) => Self::fit_to_multi_confines(multi, noise),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[derive(Debug)]
	struct MinFootprint;

	impl Fit for MinFootprint {
		fn fit_to_confines(
			confines: &Confines,
			_noise: NoiseParams,
		) -> Result<(Self, FillableRegions), FitError> {
			let fp = confines.footprint();
			if fp.x < 4.0 || fp.y < 4.0 {
				return Err(FitError::TooSmall { reason: "min_footprint" });
			}
			Ok((Self, FillableRegions::empty()))
		}
	}

	#[derive(Debug)]
	struct AlwaysInvalid;

	impl Fit for AlwaysInvalid {
		fn fit_to_confines(
			_confines: &Confines,
			_noise: NoiseParams,
		) -> Result<(Self, FillableRegions), FitError> {
			Err(FitError::InvalidConfines { reason: "bad" })
		}
	}

	fn square(side: f32) -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(side, 3.0, side)))
	}

	#[test]
	fn multi_keeps_successes_and_residuals_soft_fails() {
		let multi = MultiConfines::new([
			FillRegion::new(SpaceKind::InternalSpace, square(6.0)),
			FillRegion::new(SpaceKind::Hallway, square(2.0)),
			FillRegion::new(SpaceKind::Custom("cell".into()), square(5.0)),
		]);
		let out = MinFootprint::fit_to_multi_confines(&multi, NoiseParams::default()).unwrap();
		assert_eq!(out.fitted_len(), 2);
		assert_eq!(out.residual.within.len(), 1);
		assert_eq!(out.residual.within[0].kind, SpaceKind::Hallway);
	}

	#[test]
	fn multi_all_soft_fail_is_ok_empty_fitted() {
		let multi = MultiConfines::from_confines([square(1.0), square(2.0)]);
		let out = MinFootprint::fit_to_multi_confines(&multi, NoiseParams::default()).unwrap();
		assert!(out.is_empty());
		assert_eq!(out.residual.within.len(), 2);
	}

	#[test]
	fn multi_hard_fail_aborts() {
		let multi = MultiConfines::from_confines([square(6.0)]);
		let err = AlwaysInvalid::fit_to_multi_confines(&multi, NoiseParams::default()).unwrap_err();
		assert!(matches!(err, FitError::InvalidConfines { .. }));
	}

	#[test]
	fn fit_to_single_soft_fail_residualizes() {
		let target = FitTarget::Single(square(1.0));
		let out = MinFootprint::fit_to(&target, NoiseParams::default()).unwrap();
		assert!(out.is_empty());
		assert_eq!(out.residual.within.len(), 1);
	}
}
