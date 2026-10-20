# Controllers core

Source, actor, and UI routing plus a nested capture stack. This crate depends
only on [`maybraid-input`](../../input/README.md) and Bevy.

```text
InputSource ──► RouteTarget::Ui(UiTarget)      ──► menu consumers
            └──► RouteTarget::Actor(ActorTarget) ──► locomotion (`LocalActor`)
CaptureStack owns the UI claim for a source. Nested layers release independently.
```

Schedule contract: [`ControllerPipeline`] / [`MotorRequestStage`]. Plugin install
order is not the contract. See [ARCHITECTURE.md](../ARCHITECTURE.md) for the
role inventory and temporary adapters.
