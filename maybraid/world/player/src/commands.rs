//! Request components the world command drawer still spawns.

use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestMeshStats;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeFree;

#[derive(Component, Debug, Clone, Copy)]
pub struct RequestModeCharacter;
