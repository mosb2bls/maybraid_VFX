# Locomotion controller

Ordinary locomotion: pad [`CharacterIntent`](../character/src/intent.rs) move /
jump / sprint / stance become a [`MotorIntent`](../../player/src/motor.rs)
sample on [`InputRouter::delivered_local_actor`](../core/src/router.rs).
Disconnect and the first blocking capture emit `RoutingEvent` with
`HoldEffect::ReleaseNeutral`; the motor reads those via `MessageReader` and
drops the Standard contribution.

This crate does not apply the motor and does not rediscover a player. The
player crate composes declared contributions and writes `MoveWish` once.

See [ARCHITECTURE.md](../ARCHITECTURE.md).
