//! Circle−inscribed-square floor cap (southern hemisphere of circle minus square).

use crate::assets::panels::rough_stonework::INSCRIBED_SQUARE;
/// One 90° sector of the circle−square floor ring.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorCircleInscribedSquare;

crate::impl_glb_lod_scene!(RoughStoneFloorCircleInscribedSquare, INSCRIBED_SQUARE);
