//! Single stair tread (`rough_stonework_tread_001.glb`).

use crate::assets::stairs::rough_stonework::TREAD;
/// Unit tread cube (kit \(X = Y = Z \in [-1, 1]\), left face −Z).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneTread;

crate::impl_glb_lod_scene!(RoughStoneTread, TREAD);
