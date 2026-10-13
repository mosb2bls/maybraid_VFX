//! Shared concept-screen identifiers used by CLI, menus, and playgrounds.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ConceptAnimation {
	#[default]
	Still,
	Walk,
	Run,
	Gallop,
	Jump,
	Leap,
	Tuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
	Soaring,
	Flapping,
	Jab,
	Squat,
	Prone,
	LateralUndulation,
	DorsoventralUndulation,
}

impl ConceptAnimation {
	pub const VALUES: &'static [Self] = &[
		Self::Still,
		Self::Walk,
		Self::Run,
		Self::Gallop,
		Self::Jump,
		Self::Leap,
		Self::Tuck,
		Self::TuckedFlip,
		Self::TwoFootedTuckedFlip,
		Self::Soaring,
		Self::Flapping,
		Self::Jab,
		Self::Squat,
		Self::Prone,
		Self::LateralUndulation,
		Self::DorsoventralUndulation,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Walk => "walk",
			Self::Run => "run",
			Self::Gallop => "gallop",
			Self::Jump => "jump",
			Self::Leap => "leap",
			Self::Tuck => "tuck",
			Self::TuckedFlip => "tucked-flip",
			Self::TwoFootedTuckedFlip => "two-footed-tucked-flip",
			Self::Soaring => "soaring",
			Self::Flapping => "flapping",
			Self::Jab => "jab",
			Self::Squat => "squat",
			Self::Prone => "prone",
			Self::LateralUndulation => "lateral-undulation",
			Self::DorsoventralUndulation => "dorsoventral-undulation",
		}
	}
}
