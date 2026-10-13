//! Rory's Head-trained reuses the Storybook [`BranchOut`] phase machine ([#254](https://github.com/ramate-io/maybraid/issues/254)).

pub type RorysHeadTrainedChain = super::storybook_tree::StorybookTreeChain;

pub use super::storybook_tree::{
	is_graph_terminal, segment_fracs, storybook_branch_depth, StorybookTreePhase,
	STORYBOOK_BRANCH_DEPTH_MAX, STORYBOOK_BRANCH_DEPTH_MIN,
};
