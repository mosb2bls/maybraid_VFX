//! Floor material style.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FloorStyle {
	#[default]
	RoughStonework,
	Wood,
}
