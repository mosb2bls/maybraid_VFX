use crate::assets::floors::rough_stonework::RECTANGLE;
/// Rectangular rough stone floor fill (`rough_stonework_001.glb`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoughStoneFloorRectangle;

crate::impl_glb_lod_scene!(RoughStoneFloorRectangle, RECTANGLE);
