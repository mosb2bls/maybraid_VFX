# Controllers core

Sole [`InputRouter`](src/router.rs) for source → actor / UI assignment, plus
token-scoped capture and suspension. This crate depends only on
[`maybraid-input`](../../input/README.md) and Bevy.

```text
InputSource ──► InputRouter assignment
            ├──► RouteTarget::Ui(UiTarget)      ──► menu consumers
            └──► RouteTarget::Actor(ActorTarget) ──► character / Tanz / skill-map
Capture / suspension tokens are owner-scoped. Releasing one owner cannot
release another. Assignment loss clears delivery.
```

Controllers read the router; they never rediscover a "current player". The
lowest-ID Player fallback is resolved once at the host assignment boundary
([`InputRouter::assign_lowest_id_actor`](src/router.rs)) while the source is
connected. Reconnect is [`InputRouter::connect`](src/router.rs). Lifecycle
events are [`RoutingEvent`](src/router.rs) messages; use `MessageReader`, not
`take_events`.

Schedule contract: generic [`ControllerPipeline`](src/schedule.rs) stages.
Domain ordering lives beside domain systems. Plugin install order is not the
contract. See [ARCHITECTURE.md](../ARCHITECTURE.md) for the role inventory
and temporary adapters.
