//! Kamakura Torch reuses the Storybook [`BranchOut`] phase machine ([Kamakura torch (stashed near-vertical flame)).

pub type KamakuraTorchChain = super::storybook_tree::StorybookTreeChain;

pub use super::storybook_tree::{
	is_graph_terminal, segment_fracs, storybook_branch_depth, StorybookTreePhase,
	STORYBOOK_BRANCH_DEPTH_MAX, STORYBOOK_BRANCH_DEPTH_MIN,
};
