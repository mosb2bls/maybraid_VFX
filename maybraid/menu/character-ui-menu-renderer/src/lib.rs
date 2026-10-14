//! Maybraid HUD renderer for typed character menus.
//!
//! Menus lower into the renderer-agnostic `MenuNode` IR (see
//! `character-ui-menu`); this crate paints those nodes with mid-size HUD
//! widgets from `menu-components`. Placement is host-owned: the sink only
//! left- or right-justifies content inside the parent the host provides.
//!
//! In-screen: [`MenuFocus`] / [`MenuActivate`] / [`OverlayOpen`] / [`OverlayClose`].
//! Leave-screen: [`CharacterMenuEvent`].

pub mod event;
pub mod input;
pub mod justify;
pub mod overlay;
pub mod plugin;
pub mod sink;
pub mod widgets;

pub use event::{CharacterMenuEvent, OverlayClose, OverlayOpen};
pub use justify::MenuJustify;
pub use overlay::{
	fill_inventory_layout, find_overlay_node, flatten_nodes, is_picker_only, is_select_node,
	overlay_closes_on_pick, overlay_summary_value, primary_select, render_overlay_body,
	spawn_overlay_shell, spawn_starter_kit_shell,
};
pub use plugin::{CharacterHudSystems, MaybraidCharacterMenuRendererPlugin};
pub use sink::{MaybraidMenuSink, MenuSink, MenuThumbnailContext, NoThumbnails, RenderContext};
pub use widgets::{
	CloseOverlaySelect, MenuButton, OpenSelectKey, OverlaySelectRoot, OverlaySelectViewport,
};
