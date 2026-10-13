//! Vase Tree reuses the Storybook [`BranchOut`] phase machine ([#246](https://github.com/ramate-io/maybraid/issues/246)).

pub type VaseTreeChain = super::storybook_tree::StorybookTreeChain;

pub use super::storybook_tree::{
	is_graph_terminal, segment_fracs, stalk_tip_from_chain, storybook_branch_depth,
	StorybookTreePhase, STORYBOOK_BRANCH_DEPTH_MAX, STORYBOOK_BRANCH_DEPTH_MIN,
};
