# World controller

Owns [`SessionCapabilities`](src/capabilities.rs): actor/session participation
that is installed or removed. [`SuspensionStack`](../core/src/suspension.rs)
is a separate owned-token freeze. [`WorldGameplayEnabled`](src/enabled.rs)
is the live-play projection of installed gameplay minus gameplay suspensions.

Capabilities are independent — not a game-mode enum and not a per-frame
`SessionActivity` table. Loading streams with gameplay off. Pause writes the
pause token; motors stay installed; NPCs keep simulating. Leave / Home
uninstalls play and releases the pause token only.

Capture (U5) is a third operation: it redirects or blocks delivery without
freezing simulation. [`SessionCapabilitySystems`](src/lib.rs) applies a
pending pause after Bind and before WorldFilter / item realization.

Standard ↔ Tanzer is per-actor participation, written once on change. It
does not restamp a global mode table.
