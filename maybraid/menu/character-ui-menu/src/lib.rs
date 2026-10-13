//! Generic typed menu primitives for character creation UIs.
//!
//! This crate owns the renderer-independent data vocabulary: typed menu state
//! primitives, option traits, and the [`MenuNode`] intermediate representation
//! that menus lower into. character-specific menu instances and trait
//! implementations live in `character-creation-menus`; Bevy painting lives in
//! `bevy-character-ui-menu-renderer`.

pub mod camera_focus;
pub mod node;
pub mod primitives;
pub mod section_open;
pub mod traits;

pub use camera_focus::{CameraFocus, FocusRig};
pub use node::{
	normalize, AssetChoice, GridCatalogChoice, ItemRow, MenuComponent, MenuNode, PreviewColor,
	SelectChoice, SelectGroup, StatCard, StatLine, StatTone, SwatchChoice,
};
pub use primitives::{
	AssetSingleSelect, IdentifiedAsset, MultiSelect, Section, SingleSelect, Slider,
	SwatchSingleSelect, ThumbnailCamera,
};
pub use section_open::SectionOpen;
pub use traits::{
	AssetOption, AssetThumbnailDisplay, LabelOption, ListValues, StringIdentified, SwatchOption,
	ThumbnailRequest,
};
