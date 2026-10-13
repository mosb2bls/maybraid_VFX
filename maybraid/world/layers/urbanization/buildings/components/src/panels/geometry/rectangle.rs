//! Axis-aligned rectangular panel tile in panel space.

/// Suggested full tile width along local \(X\) (matches unscaled ground kit \(X \in [0, 1]\)).
pub const DEFAULT_TILE_WIDTH: f32 = 1.0;

/// How many tiles fit a length given a suggested width.
///
/// \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\), at least 1. Callers use
/// \(\texttt{length}/n\) as the actual tile size so tiles span the length exactly.
pub fn fitted_tile_count(length: f32, tile_width: f32) -> u32 {
	let tw = tile_width.max(1e-4);
	((length / tw).round() as i32).max(1) as u32
}

/// Kit footprint: unit square \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\)
/// (origin at the \(X{=}0\), \(Z{=}0\) corner; mesh extends along \(+X\) and \(-Z\)).
///
/// Matches the shipped `rectangle_001_*.glb` bounds (and the right-triangle Z sense).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rectangle;
