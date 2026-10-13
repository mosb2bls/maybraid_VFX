//! Semantic part slots used by character recipes and debug/UI reporting.

/// Semantic slot used for debugging and future UI/status reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CharacterPartSlot {
	#[default]
	BodyMesh,
	/// Intermediate OwnRig armature socketed between body and head (e.g. multi-bone neck).
	NeckRig,
	NeckMesh,
	HeadRig,
	HeadMesh,
	EyeLeft,
	EyeRight,
	Nose,
	Mouth,
	EarLeft,
	EarRight,
	Hair,
	Horns,
	Clothing,
	Spine,
	Tail,
}

impl CharacterPartSlot {
	/// Face, hair, and horns clip a first-person camera; body / neck / clothing do not.
	pub fn hides_in_first_person(self) -> bool {
		matches!(
			self,
			Self::HeadMesh
				| Self::Nose | Self::Mouth
				| Self::EyeLeft
				| Self::EyeRight
				| Self::EarLeft
				| Self::EarRight
				| Self::Hair | Self::Horns
		)
	}
}
