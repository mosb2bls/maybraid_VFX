//! Place the player body and its follow camera together.

use avian3d::prelude::{LinearVelocity, Position};
use bevy::prelude::*;
use player_camera::FollowCamera;

use crate::player::Player;

/// A world position and a horizontal facing for the follow camera.
pub struct PlayerSeat {
	pub at: Vec3,
	pub facing: Vec3,
}

impl PlayerSeat {
	pub fn apply(
		self,
		players: &mut Query<
			(
				&mut Transform,
				&mut GlobalTransform,
				Option<&mut Position>,
				Option<&mut LinearVelocity>,
			),
			With<Player>,
		>,
		cameras: &mut Query<
			(&mut Transform, &mut GlobalTransform, &FollowCamera),
			(With<Camera3d>, Without<Player>),
		>,
	) {
		let Ok((mut player_tf, mut player_global, mut body, mut velocity)) = players.single_mut()
		else {
			return;
		};
		player_tf.translation = self.at;
		*player_global = GlobalTransform::from(*player_tf);
		if let Some(position) = body.as_deref_mut() {
			position.0 = self.at;
		}
		if let Some(velocity) = velocity.as_deref_mut() {
			velocity.0 = Vec3::ZERO;
		}
		drop((player_tf, player_global, body, velocity));

		let Ok((mut camera_tf, mut camera_global, follow)) = cameras.single_mut() else {
			return;
		};
		let look = self.at + Vec3::Y * follow.look_height;
		let eye = look - self.facing * follow.distance + Vec3::Y * follow.height;
		let parked = Transform::from_translation(eye).looking_at(look, Vec3::Y);
		*camera_tf = parked;
		*camera_global = GlobalTransform::from(parked);
	}
}
