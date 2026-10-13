//! Shared generation-mode state.

mod generation_mode;

pub use generation_mode::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModePlugin,
	GenerationModeSystems,
};
