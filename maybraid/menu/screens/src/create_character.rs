//! Create-a-character flow: starter garments and firearms, then the body editor.

use bevy::prelude::*;
use character_items::{InventoryItem, ItemRng};
use character_persist::CharacterId;

use crate::spin_reveal::{
	cancel_spin_reveal, request_show_spin_reveal, SpinRevealFinished, SpinRevealScreenPlugin,
	SpinRevealSystems,
};

/// Queue the create-a-character flow (starter reveal, then the body HUD).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowCreateCharacter {
	pub id: CharacterId,
}

/// Starter inventory is ready; the host should open the create-mode character HUD.
#[derive(Message, Clone, Debug)]
pub struct CreateCharacterReady {
	pub id: CharacterId,
	pub items: Vec<InventoryItem>,
}

#[derive(Resource, Clone, Copy)]
struct PendingCreate(CharacterId);

pub fn request_show_create_character(commands: &mut Commands) {
	request_show_create_character_id(commands, CharacterId::new());
}

pub fn request_show_create_character_id(commands: &mut Commands, id: CharacterId) {
	commands.spawn(RequestShowCreateCharacter { id });
}

/// Drop an in-flight starter reveal so Back from spin-reveal does not create
/// and does not respawn the spinning screen after the gallery is up.
pub fn cancel_pending_create(commands: &mut Commands) {
	commands.remove_resource::<PendingCreate>();
	cancel_spin_reveal(commands);
}

pub struct CreateCharacterPlugin;

impl Plugin for CreateCharacterPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SpinRevealScreenPlugin>() {
			app.add_plugins(SpinRevealScreenPlugin);
		}
		app.add_message::<CreateCharacterReady>().add_systems(
			Update,
			(
				start_create_character.before(SpinRevealSystems::Apply),
				forward_create_character_ready.after(SpinRevealSystems::Apply),
			),
		);
	}
}

fn start_create_character(
	mut commands: Commands,
	requests: Query<(Entity, &RequestShowCreateCharacter)>,
) {
	if requests.is_empty() {
		return;
	}
	let mut id = CharacterId::new();
	for (entity, request) in &requests {
		id = request.id;
		commands.entity(entity).despawn();
	}
	commands.insert_resource(PendingCreate(id));
	let mut rng = ItemRng::from_entropy();
	let items = rng.random_starter_loadout();
	request_show_spin_reveal(&mut commands, items);
}

fn forward_create_character_ready(
	mut finished: MessageReader<SpinRevealFinished>,
	mut ready: MessageWriter<CreateCharacterReady>,
	pending: Option<Res<PendingCreate>>,
) {
	let Some(pending) = pending else {
		return;
	};
	for event in finished.read() {
		ready.write(CreateCharacterReady { id: pending.0, items: event.items.clone() });
	}
}
