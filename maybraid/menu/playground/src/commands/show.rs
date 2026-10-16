//! `/show` subcommand: spawn a menu screen.

use crate::character::request_show_character;
use crate::loading_demo::LoadingDemo;
use crate::weapon_gallery::request_show_weapons;
use bevy::prelude::*;
use clap::Subcommand;
use menu_screens::{
	request_show_create_character, request_show_gallery, request_show_home,
	request_show_in_game_with_mode, request_show_loading,
};

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Spawn the Maybraid home screen (bottom-left text menu).
	Home,
	/// Spawn the standard loading page (spinning mark, bar, explainer).
	Loading,
	/// Spawn the Maybraid character-creator panel (right-justified HUD).
	Character,
	/// Starter Kit, then the humanoid create-a-character HUD.
	CreateCharacter,
	/// Saved-character gallery (new or open).
	Gallery,
	/// Generated firearm kits on a handheld-scale grid.
	Weapons,
	/// Spawn the in-game pause menu (actions plus Maybraid - mode).
	InGame {
		/// Label after Maybraid in the upper-left title.
		#[arg(default_value = "Discovery")]
		mode: String,
	},
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Show::Home => request_show_home(commands),
			Show::Loading => {
				request_show_loading(commands);
				commands.insert_resource(LoadingDemo::default());
			}
			Show::Character => request_show_character(commands),
			Show::CreateCharacter => request_show_create_character(commands),
			Show::Gallery => request_show_gallery(commands),
			Show::Weapons => request_show_weapons(commands),
			Show::InGame { mode } => request_show_in_game_with_mode(commands, mode),
		}
	}
}
