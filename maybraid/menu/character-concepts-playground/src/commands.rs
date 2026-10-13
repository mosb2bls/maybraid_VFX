//! In-game clap command hierarchy for the concepts playground.

pub mod braidman;
pub mod brenal;
pub mod brodler;
pub mod brokker;
pub mod caole;
pub mod chupri;
pub mod claber;
pub mod croconot;
pub mod dui;
pub mod epiphant;
pub mod grener;
pub mod hars;
pub mod kaller;
pub mod kappler;
pub mod kispar;
pub mod lero;
pub mod lidder;
pub mod mistler;
pub mod mygr;
pub mod sonyak;
pub mod spibmom;
pub mod tapp;
pub mod thumplus;
pub mod tipple;
pub mod topple;
pub mod tuberwaber;
pub mod wumbus;
pub mod ylter;

use bevy::prelude::*;
pub use braidman::Braidman;
pub use brenal::Brenal;
pub use brodler::Brodler;
pub use brokker::Brokker;
pub use caole::Caole;
pub use chupri::Chupri;
pub use claber::Claber;
use clap::Parser;
pub use croconot::Croconot;
pub use dui::Dui;
pub use epiphant::Epiphant;
use game_commands::command::{CommandScript, GameCommand};
pub use grener::Grener;
pub use hars::Hars;
pub use kaller::Kaller;
pub use kappler::Kappler;
pub use kispar::Kispar;
pub use lero::Lero;
pub use lidder::Lidder;
pub use mistler::Mistler;
pub use mygr::Mygr;
pub use sonyak::Sonyak;
pub use spibmom::Spibmom;
pub use tapp::Tapp;
pub use thumplus::Thumplus;
pub use tipple::Tipple;
pub use topple::Topple;
pub use tuberwaber::Tuberwaber;
pub use wumbus::Wumbus;
pub use ylter::Yilter;

use crate::skinning::request_dump_bones;

pub const CONCEPTS_CLI_NAME: &str = "character-concepts";
pub type Script = CommandScript<ConceptsCommand>;

#[derive(Clone, Parser, Component)]
#[command(
	name = "character-concepts",
	version,
	about = "character concepts playground commands",
	rename_all = "kebab-case",
	disable_help_subcommand = true
)]
pub enum ConceptsCommand {
	Help,
	Script(Script),
	/// Spawn or adjust the simplified Braidman concept preview.
	#[command(subcommand)]
	Braidman(Braidman),
	/// Spawn or adjust the Brenal quadruped concept preview.
	#[command(subcommand)]
	Brenal(Brenal),
	/// Spawn or adjust the Caole quadruped concept preview.
	#[command(subcommand)]
	Caole(Caole),
	/// Spawn or adjust the Epiphant elephant-like quadruped concept preview.
	#[command(subcommand)]
	Epiphant(Epiphant),
	/// Spawn or adjust the Hars horse-like quadruped concept preview.
	#[command(subcommand)]
	Hars(Hars),
	/// Spawn or adjust the Yilter long-necked quadruped concept preview.
	#[command(subcommand)]
	Yilter(Yilter),
	/// Spawn or adjust the Sonyak Gumbus-bodied quadruped concept preview.
	#[command(subcommand)]
	Sonyak(Sonyak),
	/// Spawn or adjust the Claber oversized low-slung quadruped concept preview.
	#[command(subcommand)]
	Claber(Claber),
	/// Spawn or adjust the Croconot low-slung quadruped concept preview.
	#[command(subcommand)]
	Croconot(Croconot),
	/// Spawn or adjust the Brodler concept preview.
	#[command(subcommand)]
	Brodler(Brodler),
	/// Spawn or adjust the Mygr concept preview.
	#[command(subcommand)]
	Mygr(Mygr),
	/// Spawn or adjust the Dui concept preview.
	#[command(subcommand)]
	Dui(Dui),
	/// Spawn or adjust the Lidder concept preview.
	#[command(subcommand)]
	Lidder(Lidder),
	/// Spawn or adjust the Chupri concept preview.
	#[command(subcommand)]
	Chupri(Chupri),
	/// Spawn or adjust the Brokker concept preview.
	#[command(subcommand)]
	Brokker(Brokker),
	/// Spawn or adjust the Tipple concept preview.
	#[command(subcommand)]
	Tipple(Tipple),
	/// Spawn or adjust the Topple concept preview.
	#[command(subcommand)]
	Topple(Topple),
	/// Spawn or adjust the Kispar concept preview.
	#[command(subcommand)]
	Kispar(Kispar),
	/// Spawn or adjust the Tapp concept preview.
	#[command(subcommand)]
	Tapp(Tapp),
	/// Spawn or adjust the Kaller concept preview.
	#[command(subcommand)]
	Kaller(Kaller),
	/// Spawn or adjust the Kappler concept preview.
	#[command(subcommand)]
	Kappler(Kappler),
	/// Spawn or adjust the Wumbus concept preview.
	#[command(subcommand)]
	Wumbus(Wumbus),
	/// Spawn or adjust the Lero concept preview.
	#[command(subcommand)]
	Lero(Lero),
	/// Spawn or adjust the Spibmom concept preview.
	#[command(subcommand)]
	Spibmom(Spibmom),
	/// Spawn or adjust the Grener shark concept preview.
	#[command(subcommand)]
	Grener(Grener),
	/// Spawn or adjust the Thumplus whale concept preview.
	#[command(subcommand)]
	Thumplus(Thumplus),
	/// Spawn or adjust the Mistler sprite-fish concept preview.
	#[command(subcommand)]
	Mistler(Mistler),
	/// Spawn or adjust the Tuberwaber biped concept preview.
	#[command(subcommand)]
	Tuberwaber(Tuberwaber),
	/// Print the live rig bone hierarchy to the HUD console.
	DumpBones,
}

impl ConceptsCommand {
	pub fn long_help_string() -> String {
		<Self as GameCommand>::long_help_string()
	}

	pub fn parse_startup_command() -> Result<Option<Self>, String> {
		<Self as GameCommand>::parse_startup_command()
	}

	pub fn react(self, commands: &mut Commands, console: &mut String) {
		match self {
			Self::Help => *console = Self::long_help_string(),
			Self::Script(script) => script.run(commands, console),
			Self::Braidman(braidman) => braidman.react(commands),
			Self::Brenal(brenal) => brenal.react(commands),
			Self::Caole(caole) => caole.react(commands),
			Self::Epiphant(epiphant) => epiphant.react(commands),
			Self::Hars(hars) => hars.react(commands),
			Self::Yilter(ylter) => ylter.react(commands),
			Self::Sonyak(sonyak) => sonyak.react(commands),
			Self::Claber(claber) => claber.react(commands),
			Self::Croconot(croconot) => croconot.react(commands),
			Self::Brodler(brodler) => brodler.react(commands),
			Self::Mygr(mygr) => mygr.react(commands),
			Self::Dui(dui) => dui.react(commands),
			Self::Lidder(lidder) => lidder.react(commands),
			Self::Chupri(chupri) => chupri.react(commands),
			Self::Brokker(brokker) => brokker.react(commands),
			Self::Tipple(tipple) => tipple.react(commands),
			Self::Topple(topple) => topple.react(commands),
			Self::Kispar(kispar) => kispar.react(commands),
			Self::Tapp(tapp) => tapp.react(commands),
			Self::Kaller(kaller) => kaller.react(commands),
			Self::Kappler(kappler) => kappler.react(commands),
			Self::Wumbus(wumbus) => wumbus.react(commands),
			Self::Lero(lero) => lero.react(commands),
			Self::Spibmom(spibmom) => spibmom.react(commands),
			Self::Grener(grener) => grener.react(commands),
			Self::Thumplus(thumplus) => thumplus.react(commands),
			Self::Mistler(mistler) => mistler.react(commands),
			Self::Tuberwaber(tuberwaber) => tuberwaber.react(commands),
			Self::DumpBones => request_dump_bones(commands),
		}
	}
}

impl GameCommand for ConceptsCommand {
	const CLI_NAME: &'static str = CONCEPTS_CLI_NAME;

	fn react(self, commands: &mut Commands, console: &mut String) {
		Self::react(self, commands, console);
	}
}
