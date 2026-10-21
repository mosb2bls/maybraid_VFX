# Menu controller

Routes [`MenuNavPad`](../../input/README.md) impulses onto one focused menu
inside a scope entity. Put [`MenuController`] on a screen root; descendants
that are `TextMenu` / `HudMenu` are eligible. Overlay menus win over panels.

Delivery is a [`MenuNavImpulse`] on a [`UiTarget`](../core/src/ui.rs) — not a
broadcast to every child. Exclusive modal nav acquires a
[`CaptureToken`](../core/src/capture.rs) on
[`InputRouter`](../core/src/router.rs); widgets apply the impulse;
`MenuActivate` still bubbles up.

This is the phase 1 targeted path. See [ARCHITECTURE.md](../ARCHITECTURE.md).

**B** remains `MenuNav::Back`. Character stance also reads **B** (tap squat /
hold prone) but only when no `MenuController` has focus.
