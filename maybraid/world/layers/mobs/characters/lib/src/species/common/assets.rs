//! Shared humanoid asset paths and mesh enums used by multiple species.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::assets::AssetPath;

pub const BODY_RIG: AssetPath = AssetPath::new("characters/bodies/biped/humanoid_rig.glb");
pub const QUADRUPED_RIG: AssetPath =
	AssetPath::new("characters/bodies/quadruped/quadruped_rig.glb");
pub const FORELIMBED_RIG: AssetPath =
	AssetPath::new("characters/bodies/forelimbed/forelimbed_rig.glb");
pub const BODY_SHARK: AssetPath = AssetPath::new("characters/bodies/forelimbed/shark.glb");
pub const BODY_WHALE: AssetPath = AssetPath::new("characters/bodies/forelimbed/whale.glb");
pub const BODY_SPRITE_FISH: AssetPath =
	AssetPath::new("characters/bodies/forelimbed/sprite_fish.glb");
pub const BODY_GUMBUS: AssetPath =
	AssetPath::new("characters/bodies/quadruped/gumbus_quadruped_full_body.glb");
pub const BODY_DRAGLOON: AssetPath =
	AssetPath::new("characters/bodies/quadruped/dragloon_quadruped_full_body.glb");
pub const BODY_RUMBLER: AssetPath = AssetPath::new("characters/bodies/quadruped/rumbler.glb");
pub const PRONOGRADE_HEAD_RIG: AssetPath = AssetPath::new("characters/heads/pronograde_head.glb");
/// Pronograde canine head mesh (`bear_head.glb` shares the pronograde rig layout).
pub const HEAD_CANINE: AssetPath = AssetPath::new("characters/heads/bear_head.glb");
/// Pronograde Caole head mesh.
pub const HEAD_CAOLE: AssetPath = AssetPath::new("characters/heads/caole.glb");
pub const HEAD_COWDER: AssetPath = AssetPath::new("characters/heads/cowder.glb");
/// Triple-joint neck armature only (`neck_base` → `mid_neck` → `upper_neck` → `head_socket`).
pub const NECK_TRIPLE_JOIN: AssetPath = AssetPath::new("characters/necks/triple_join_3_1.glb");
/// Skinned neck mesh authored against the triple-join joint names.
pub const NECK_BASIC: AssetPath = AssetPath::new("characters/necks/basic_3_1.glb");
pub const TAIL_CAT: AssetPath = AssetPath::new("characters/tails/cat_tail.glb");
pub const TAIL_LERODON: AssetPath = AssetPath::new("characters/tails/lerodon_tail.glb");
pub const TAIL_LERODON_QUADRUPED: AssetPath =
	AssetPath::new("characters/tails/lerodon_tail_quadruped.glb");
pub const BODY_STANDARD: AssetPath =
	AssetPath::new("characters/bodies/biped/humanoid_full_body.glb");
pub const BODY_FULL: AssetPath =
	AssetPath::new("characters/bodies/biped/leron_biped_full_body.glb");
pub const HEAD_RIG: AssetPath = AssetPath::new("characters/heads/orthograde_head.glb");
pub const HEAD_STANDARD: AssetPath = AssetPath::new("characters/heads/meerkat_head_v2.glb");
pub const HEAD_STANDARD_PRONOGRADE: AssetPath =
	AssetPath::new("characters/heads/meerkat_head_pronograde.glb");
pub const HEAD_GAUNT: AssetPath = AssetPath::new("characters/heads/gaunt_ortho_humanoid_head.glb");
pub const HEAD_FULL: AssetPath = AssetPath::new("characters/heads/full_ortho_humanoid_head.glb");
pub const HEAD_ORTHO_BEAR: AssetPath = AssetPath::new("characters/heads/ortho_bear_head.glb");
pub const EYE_STANDARD: AssetPath = AssetPath::new("characters/eyes/humanoid_eye_left.glb");
pub const EYE_FALCON: AssetPath = AssetPath::new("characters/eyes/falcon_eye_left.glb");
pub const NOSE_STANDARD: AssetPath = AssetPath::new("characters/noses/humanoid_nose.glb");
pub const NOSE_BROAD: AssetPath = AssetPath::new("characters/noses/broad_humanoid_nose.glb");
pub const NOSE_LOAF: AssetPath = AssetPath::new("characters/noses/loaf_nose.glb");
pub const NOSE_BALLOON: AssetPath = AssetPath::new("characters/noses/mumbus_nose.glb");
pub const MOUTH_STANDARD: AssetPath = AssetPath::new("characters/mouths/common_mouth.glb");
pub const MOUTH_CANINE_SNOUT: AssetPath = AssetPath::new("characters/snouts/canine.glb");
pub const MOUTH_LERODON_SNOUT: AssetPath = AssetPath::new("characters/snouts/lerodon_snout.glb");
pub const MOUTH_ROBREK_SNOUT: AssetPath = AssetPath::new("characters/snouts/robrek_snout.glb");
pub const MOUTH_COW_SNOUT: AssetPath = AssetPath::new("characters/snouts/cow.glb");
pub const EAR_STANDARD: AssetPath =
	AssetPath::new("characters/ears/round_scoop_lateral_ear_left.glb");
pub const EAR_ROUND: AssetPath = AssetPath::new("characters/ears/round_lateral_ear_left.glb");
pub const EAR_FLANK: AssetPath = AssetPath::new("characters/ears/flank_lateral_ear_left.glb");
pub const HORNS_HARROWED_CROWN: AssetPath = AssetPath::new("characters/horns/harrowed_crown.glb");
pub const HORNS_LORKEN_CROWN: AssetPath = AssetPath::new("characters/horns/lorken_crown.glb");
pub const HAIR_THICK_BRAIDS: AssetPath = AssetPath::new("characters/hair/thick_braids.glb");
pub const HAIR_FLOWING_CURLS: AssetPath = AssetPath::new("characters/hair/flowing_curls.glb");
pub const HAIR_WRAPPING_BRAIDS: AssetPath = AssetPath::new("characters/hair/wrapping_braids.glb");
pub const HAIR_WRAPPING_BRAIDS_HANGING_LOCKS: AssetPath =
	AssetPath::new("characters/hair/wrapping_braids_hanging_locks.glb");
pub const HAIR_BRAID_HAWK: AssetPath = AssetPath::new("characters/hair/braid_hawk.glb");
pub const HAIR_FEATHER_HAWK: AssetPath = AssetPath::new("characters/hair/feather_hawk.glb");
pub const HAIR_FLOWING_EDGY_CURLS: AssetPath =
	AssetPath::new("characters/hair/flowing_edgy_curls.glb");
pub const HAIR_PERM_BRAID: AssetPath = AssetPath::new("characters/hair/perm_braid.glb");
pub const HAIR_TECHNO_EDGE: AssetPath = AssetPath::new("characters/hair/techno_edge.glb");
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BodyMesh {
	#[default]
	Standard,
	Full,
}

impl BodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Full];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Full => "full",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => BODY_STANDARD,
			Self::Full => BODY_FULL,
		}
	}

	pub const fn clothing_host(self) -> character_items::ClothingHost {
		use character_items::ClothingHost;
		match self {
			Self::Standard => ClothingHost::HUMANOID,
			Self::Full => ClothingHost::LERON,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum HeadMesh {
	#[default]
	Standard,
	Gaunt,
	Full,
}

impl HeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Gaunt, Self::Full];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Gaunt => "gaunt",
			Self::Full => "full",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => HEAD_STANDARD,
			Self::Gaunt => HEAD_GAUNT,
			Self::Full => HEAD_FULL,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum EyeMesh {
	#[default]
	Standard,
	Falcon,
}

impl EyeMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Falcon];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Falcon => "falcon",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EYE_STANDARD,
			Self::Falcon => EYE_FALCON,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum NoseMesh {
	#[default]
	Standard,
	Broad,
	Loaf,
	Balloon,
}

impl NoseMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Broad, Self::Loaf, Self::Balloon];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Broad => "broad",
			Self::Loaf => "loaf",
			Self::Balloon => "balloon",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => NOSE_STANDARD,
			Self::Broad => NOSE_BROAD,
			Self::Loaf => NOSE_LOAF,
			Self::Balloon => NOSE_BALLOON,
		}
	}

	pub const fn normalization(self) -> crate::assets::AssetNormalization {
		use crate::assets::AssetNormalization;
		match self {
			Self::Balloon => AssetNormalization::centroid(0.08),
			Self::Standard | Self::Broad | Self::Loaf => AssetNormalization::centroid(0.2),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum MouthMesh {
	#[default]
	Standard,
}

impl MouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard];

	pub const fn label(self) -> &'static str {
		"standard"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_STANDARD
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum EarMesh {
	#[default]
	Standard,
	Round,
	Flank,
}

impl EarMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard, Self::Round, Self::Flank];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "standard",
			Self::Round => "round",
			Self::Flank => "flank",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Standard => EAR_STANDARD,
			Self::Round => EAR_ROUND,
			Self::Flank => EAR_FLANK,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum HairMesh {
	#[default]
	None,
	ThickBraids,
	FlowingCurls,
	WrappingBraids,
	WrappingBraidsHangingLocks,
	BraidHawk,
	FeatherHawk,
	FlowingEdgyCurls,
	PermBraid,
	TechnoEdge,
}

impl HairMesh {
	pub const VALUES: &'static [Self] = &[
		Self::None,
		Self::ThickBraids,
		Self::FlowingCurls,
		Self::WrappingBraids,
		Self::WrappingBraidsHangingLocks,
		Self::BraidHawk,
		Self::FeatherHawk,
		Self::FlowingEdgyCurls,
		Self::PermBraid,
		Self::TechnoEdge,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::ThickBraids => "thick-braids",
			Self::FlowingCurls => "flowing-curls",
			Self::WrappingBraids => "wrapping-braids",
			Self::WrappingBraidsHangingLocks => "wrapping-braids-hanging-locks",
			Self::BraidHawk => "braid-hawk",
			Self::FeatherHawk => "feather-hawk",
			Self::FlowingEdgyCurls => "flowing-edgy-curls",
			Self::PermBraid => "perm-braid",
			Self::TechnoEdge => "techno-edge",
		}
	}

	pub const fn path(self) -> Option<AssetPath> {
		match self {
			Self::None => None,
			Self::ThickBraids => Some(HAIR_THICK_BRAIDS),
			Self::FlowingCurls => Some(HAIR_FLOWING_CURLS),
			Self::WrappingBraids => Some(HAIR_WRAPPING_BRAIDS),
			Self::WrappingBraidsHangingLocks => Some(HAIR_WRAPPING_BRAIDS_HANGING_LOCKS),
			Self::BraidHawk => Some(HAIR_BRAID_HAWK),
			Self::FeatherHawk => Some(HAIR_FEATHER_HAWK),
			Self::FlowingEdgyCurls => Some(HAIR_FLOWING_EDGY_CURLS),
			Self::PermBraid => Some(HAIR_PERM_BRAID),
			Self::TechnoEdge => Some(HAIR_TECHNO_EDGE),
		}
	}
}
