//! Stair material style.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StairStyle {
	#[default]
	RoughStonework,
	Wood,
}
