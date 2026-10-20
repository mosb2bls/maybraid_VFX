# Locomotion controller

Ordinary locomotion: pad [`CharacterIntent`](../character/src/intent.rs) move /
jump / sprint / stance become a [`MotorIntent`](../../player/src/motor.rs)
sample on one [`ActorTarget`](../core/src/actor.rs).

This crate does not apply the motor. The player crate resolves declared
slots (Standard, Tanz, NPC, recovery, separation, world suppress) and writes
`MoveWish` once.

See [ARCHITECTURE.md](../ARCHITECTURE.md).
