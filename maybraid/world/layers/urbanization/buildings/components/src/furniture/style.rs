//! Furniture material / look.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FurnitureStyle {
	/// Color-coded wireframe placeholders until GLB kits exist.
	#[default]
	Placeholder,
}
