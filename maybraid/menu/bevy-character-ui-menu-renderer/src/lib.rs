//! Bevy renderer for typed character menus.
//!
//! Menus lower into the renderer-agnostic `MenuNode` IR (see
//! `character-ui-menu`); this crate paints those nodes via [`BevyMenuSink`]
//! and forwards widget presses back as the host event embedded in each leaf.

pub mod event;
pub mod plugin;
pub mod sink;
pub mod widgets;

pub use event::CharacterMenuEvent;
pub use plugin::CharacterMenuRendererPlugin;
pub use sink::{BevyMenuSink, MenuSink, MenuThumbnailContext, RenderContext};
pub use widgets::{MenuButton, ToggleSectionKey};
