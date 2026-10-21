# Locomotion controller

Ordinary locomotion: pad [`CharacterIntent`](../character/src/intent.rs) move /
jump / sprint / stance become a [`MotorIntent`](../../player/src/motor.rs)
sample on [`InputRouter::local_actor`](../core/src/router.rs).

This crate does not apply the motor and does not rediscover a player. The
player crate composes declared contributions and writes `MoveWish` once.
Tanz and skill-map consume the same Bind-stage [`RoutedActorControl`](../character/src/sample.rs)
from the character controller; locomotion does not re-route the pad.

See [ARCHITECTURE.md](../ARCHITECTURE.md).
