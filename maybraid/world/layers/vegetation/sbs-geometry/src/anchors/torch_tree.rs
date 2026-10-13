//! Shared vase-torch construction defaults ([`PenmarchTorch`](super::penmarch_torch), [`KamakuraTorch`](super::kamakura_torch)).

/// World-height fraction between ring planes along the stalk (RFC ~`0.08 H`).
pub const TORCH_RING_SPACING_WORLD_FRACTION: f32 = 0.08;

/// Ring spacing as a stalk-unit fraction.
pub fn torch_ring_spacing_unit_height(stalk_height_fraction: f32) -> f32 {
	TORCH_RING_SPACING_WORLD_FRACTION / stalk_height_fraction
}

/// Highest ring along the stalk (unit height fraction; tip = 1).
pub const TORCH_LAST_RING_UNIT_HEIGHT: f32 = 1.0;

pub const TORCH_ANCHORS_PER_RING: u32 = 6;
pub const TORCH_BRANCH_DEPTH: usize = 4;
pub const TORCH_CHILD_COUNT_MIN: u32 = 1;
pub const TORCH_CHILD_COUNT_MAX: u32 = 3;
pub const TORCH_BIAS_BLEND: f32 = 1.0;

pub const TORCH_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.12;
pub const TORCH_BRANCH_RADIUS_CHILD_SCALE_LO: f32 = 0.75;
pub const TORCH_BRANCH_RADIUS_CHILD_SCALE_HI: f32 = 0.82;

/// Terminal leaf ball radius as a fraction of tree height.
pub const TORCH_LEAF_RADIUS_FRACTION: f32 = 0.06;

/// Radial offset of ring seeds from the stalk centroid, as a fraction of stalk base radius.
pub const TORCH_RADIAL_OFFSET_FRACTION_OF_STALK_BASE: f32 = 0.05;
pub const TORCH_LIMB_BASE_RADIUS_FLOOR: f32 = 0.02;

/// First limb segment length jitter relative to the vase projection at the ring.
pub const TORCH_FIRST_SEGMENT_LENGTH_LO: f32 = 0.97;
pub const TORCH_FIRST_SEGMENT_LENGTH_HI: f32 = 1.03;

/// Branch hysteresis noise frequency multiplier at ring seeds.
pub const TORCH_BRANCH_HYSTERESIS_FREQUENCY_SCALE: f32 = 10.0;

pub const TORCH_RING_HEIGHT_EPSILON: f32 = 1e-6;
pub const TORCH_STALK_RADIUS_EPSILON: f32 = 1e-4;
pub const TORCH_RADIAL_DIRECTION_EPSILON: f32 = 1e-12;

pub const TORCH_ANCHOR_VERTICAL_OFFSET_LO: f32 = -1.0;
pub const TORCH_ANCHOR_VERTICAL_OFFSET_HI: f32 = 1.0;
pub const TORCH_ANCHOR_ANGULAR_SCALE_LO: f32 = 0.0;
pub const TORCH_ANCHOR_ANGULAR_SCALE_HI: f32 = 0.5;
pub const TORCH_ANCHOR_RADIUS_OFFSET_LO: f32 = -0.05;
pub const TORCH_ANCHOR_RADIUS_OFFSET_HI: f32 = 0.05;
