//! Character id and on-disk save directories.
//!
//! Default root is `<repo>/.maybraid/saves` in a checkout ([`SaveRoot::workspace`]).
//! Packaged builds use [`SaveRoot::discover`] (user data, or `MAYBRAID_SAVES`).
//! Character appearance, inventory, and position files live in sibling folders
//! keyed by the same [`CharacterId`].

use bevy::prelude::*;
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const CHARACTERS_DIR: &str = "characters";
const INVENTORIES_DIR: &str = "inventories";
const POSITIONS_DIR: &str = "positions";
const ACTIVE_FILE: &str = "active.json";
const SETTINGS_FILE: &str = "settings.json";

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Join key for a saved character and every User that hangs off it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterId(pub u128);

impl CharacterId {
	pub fn new() -> Self {
		let nanos = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos() as u64)
			.unwrap_or(0x9E37_79B9_7F4A_7C15);
		let mix = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
		let a = nanos.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ mix.wrapping_mul(0xBF58_476D_1CE4_E5B9);
		let b = nanos.rotate_left(17) ^ mix.wrapping_mul(0x94D0_49BB_1331_11EB);
		Self(((a as u128) << 64) | b as u128)
	}

	pub fn to_hex(self) -> String {
		format!("{:032x}", self.0)
	}

	pub fn from_hex(value: &str) -> Option<Self> {
		if value.len() != 32 {
			return None;
		}
		u128::from_str_radix(value, 16).ok().map(Self)
	}
}

impl Default for CharacterId {
	fn default() -> Self {
		Self(0)
	}
}

impl Serialize for CharacterId {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_hex())
	}
}

impl<'de> Deserialize<'de> for CharacterId {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let value = String::deserialize(deserializer)?;
		Self::from_hex(&value).ok_or_else(|| DeError::custom("invalid character id"))
	}
}

/// Root of `characters/`, `inventories/`, `positions/`, `active.json`, and
/// machine-scoped `settings.json`.
#[derive(Resource, Clone, Debug)]
pub struct SaveRoot {
	pub path: PathBuf,
}

impl SaveRoot {
	pub fn at(path: impl Into<PathBuf>) -> Self {
		Self { path: path.into() }
	}

	/// Repo `.maybraid/saves`, from this crate's manifest directory.
	pub fn workspace() -> Self {
		Self {
			path: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../../../.maybraid/saves"),
		}
	}

	/// Packaged install → user data; otherwise [`Self::workspace`].
	///
	/// `MAYBRAID_SAVES` wins. A macOS `.app`, a sidecar `assets/` next to the
	/// exe (outside Cargo `target/`), or `MAYBRAID_PACKAGED` selects the
	/// platform user-data directory (`io.ramate.maybraid`).
	pub fn discover() -> Self {
		if let Ok(path) = env::var("MAYBRAID_SAVES") {
			return Self::at(path);
		}
		if packaged_install() {
			return Self::at(user_saves_dir());
		}
		Self::workspace()
	}

	pub fn characters_dir(&self) -> PathBuf {
		self.path.join(CHARACTERS_DIR)
	}

	pub fn inventories_dir(&self) -> PathBuf {
		self.path.join(INVENTORIES_DIR)
	}

	pub fn character_path(&self, id: CharacterId) -> PathBuf {
		self.characters_dir().join(format!("{}.json", id.to_hex()))
	}

	pub fn inventory_path(&self, id: CharacterId) -> PathBuf {
		self.inventories_dir().join(format!("{}.json", id.to_hex()))
	}

	pub fn positions_dir(&self) -> PathBuf {
		self.path.join(POSITIONS_DIR)
	}

	pub fn position_path(&self, id: CharacterId) -> PathBuf {
		self.positions_dir().join(format!("{}.json", id.to_hex()))
	}

	pub fn active_path(&self) -> PathBuf {
		self.path.join(ACTIVE_FILE)
	}

	pub fn settings_path(&self) -> PathBuf {
		self.path.join(SETTINGS_FILE)
	}

	pub fn ensure_dirs(&self) -> io::Result<()> {
		fs::create_dir_all(self.characters_dir())?;
		fs::create_dir_all(self.inventories_dir())?;
		fs::create_dir_all(self.positions_dir())?;
		Ok(())
	}

	pub fn list_ids(&self) -> io::Result<Vec<CharacterId>> {
		let mut ids = Vec::new();
		let dir = self.characters_dir();
		let entries = match fs::read_dir(&dir) {
			Ok(entries) => entries,
			Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ids),
			Err(error) => return Err(error),
		};
		for entry in entries {
			let entry = entry?;
			let path = entry.path();
			if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
				continue;
			}
			let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
				continue;
			};
			if let Some(id) = CharacterId::from_hex(stem) {
				ids.push(id);
			}
		}
		ids.sort_by_key(|id| id.0);
		Ok(ids)
	}

	/// Removes appearance, inventory, and position files when present.
	pub fn delete(&self, id: CharacterId) -> io::Result<()> {
		remove_if_exists(&self.character_path(id))?;
		remove_if_exists(&self.inventory_path(id))?;
		remove_if_exists(&self.position_path(id))?;
		Ok(())
	}
}

#[cfg(target_os = "macos")]
const APP_DATA_DIR: &str = "io.ramate.maybraid";

fn packaged_install() -> bool {
	env::var_os("MAYBRAID_PACKAGED").is_some()
		|| env::current_exe().ok().is_some_and(|exe| executable_is_packaged(&exe))
}

fn executable_is_packaged(exe: &Path) -> bool {
	is_macos_bundle(exe) || sidecar_assets_outside_cargo_target(exe)
}

fn is_macos_bundle(exe: &Path) -> bool {
	let Some(macos) = exe.parent() else {
		return false;
	};
	macos.file_name() == Some("MacOS".as_ref())
		&& macos.parent().and_then(|contents| contents.file_name()) == Some("Contents".as_ref())
}

fn sidecar_assets_outside_cargo_target(exe: &Path) -> bool {
	let Some(dir) = exe.parent() else {
		return false;
	};
	dir.join("assets").is_dir()
		&& !exe.components().any(|component| component.as_os_str() == "target")
}

fn user_saves_dir() -> PathBuf {
	#[cfg(target_os = "macos")]
	{
		home_dir().join("Library/Application Support").join(APP_DATA_DIR).join("saves")
	}
	#[cfg(target_os = "windows")]
	{
		env::var_os("APPDATA")
			.map(PathBuf::from)
			.unwrap_or_else(home_dir)
			.join("Maybraid")
			.join("saves")
	}
	#[cfg(not(any(target_os = "macos", target_os = "windows")))]
	{
		env::var_os("XDG_DATA_HOME")
			.map(PathBuf::from)
			.unwrap_or_else(|| home_dir().join(".local/share"))
			.join("maybraid")
			.join("saves")
	}
}

fn home_dir() -> PathBuf {
	env::var_os("HOME")
		.or_else(|| env::var_os("USERPROFILE"))
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("."))
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
	match fs::remove_file(path) {
		Ok(()) => Ok(()),
		Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
		Err(error) => Err(error),
	}
}

#[derive(Serialize, Deserialize)]
struct ActiveFile {
	version: u32,
	id: CharacterId,
}

/// Write the id shown on home and in the gallery pane.
pub fn save_active(root: &SaveRoot, id: CharacterId) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let json = serde_json::to_string_pretty(&ActiveFile { version: 1, id })?;
	fs::write(root.active_path(), json)?;
	Ok(())
}

pub fn load_active(root: &SaveRoot) -> Option<CharacterId> {
	let json = fs::read_to_string(root.active_path()).ok()?;
	let file: ActiveFile = serde_json::from_str(&json).ok()?;
	Some(file.id)
}

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
	#[error(transparent)]
	Io(#[from] io::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hex_round_trips() {
		let id = CharacterId(0x0123_4567_89AB_CDEF_FEDC_BA98_7654_3210);
		assert_eq!(id.to_hex().len(), 32);
		assert_eq!(CharacterId::from_hex(&id.to_hex()), Some(id));
		assert_eq!(CharacterId::from_hex("short"), None);
	}

	#[test]
	fn list_and_delete_use_character_files() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		root.ensure_dirs().expect("dirs");
		let id = CharacterId(1);
		fs::write(root.character_path(id), "{}").expect("write");
		fs::write(root.inventory_path(id), "{}").expect("write");
		fs::write(root.position_path(id), "{}").expect("write");
		assert_eq!(root.list_ids().expect("list"), vec![id]);
		root.delete(id).expect("delete");
		assert!(root.list_ids().expect("list").is_empty());
		assert!(!root.character_path(id).exists());
		assert!(!root.inventory_path(id).exists());
		assert!(!root.position_path(id).exists());
	}

	#[test]
	fn active_id_round_trips() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		save_active(&root, id).expect("save");
		assert_eq!(load_active(&root), Some(id));
	}

	#[test]
	fn settings_file_sits_beside_active() {
		let root = SaveRoot::at("saves");
		assert_eq!(root.settings_path(), root.path.join("settings.json"));
		assert_eq!(root.settings_path().parent(), root.active_path().parent());
	}

	#[test]
	fn macos_bundle_exe_is_packaged() {
		let exe = Path::new("/Applications/Maybraid.app/Contents/MacOS/maybraid");
		assert!(is_macos_bundle(exe));
		assert!(executable_is_packaged(exe));
		assert!(!is_macos_bundle(Path::new("/tmp/target/release/maybraid")));
	}

	#[test]
	fn cargo_target_sidecar_is_not_packaged() {
		let dir = tempfile::tempdir().expect("tempdir");
		let target = dir.path().join("target").join("release");
		fs::create_dir_all(target.join("assets")).expect("assets");
		let exe = target.join("maybraid");
		assert!(!sidecar_assets_outside_cargo_target(&exe));
	}
}
