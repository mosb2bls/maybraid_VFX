//! Standard [`material_ref::MaterialRef`] identifiers for vegetation.

use material_ref::MaterialRef;

/// Named recipe resolved to leaf material by playground / domain libs.
pub const LEAF_MATERIAL: &str = "CHICO_LEAF_MATERIAL";

/// Named recipe resolved to stick material by playground / domain libs.
pub const STICK_MATERIAL: &str = "CHICO_STICK_MATERIAL";

/// Named recipe resolved to frond material by playground / domain libs.
pub const FROND_MATERIAL: &str = "CHICO_FROND_MATERIAL";

/// Named leaf shader recipe for higher-order canopy types (e.g. braid oak).
pub fn leaf_material_ref() -> MaterialRef {
	MaterialRef::named(LEAF_MATERIAL)
}

/// Named stick shader recipe for higher-order stick / trunk types.
pub fn stick_material_ref() -> MaterialRef {
	MaterialRef::named(STICK_MATERIAL)
}

/// Named frond shader recipe (palette + sway, no leaf cheese).
pub fn frond_material_ref() -> MaterialRef {
	MaterialRef::named(FROND_MATERIAL)
}
