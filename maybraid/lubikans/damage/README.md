# Lubikans damage bridge

Attaches [`LubikanAccountBinding`](../src/ledger.rs) onto the generic
[`DamageCredit`](../../damage/src/attribution.rs) seam so
[`damage`](../../damage/) does not depend on [`lubikans`](../).

`LubikanDamagePlugin` copies a changed binding onto `DamageCredit` before
`DamageSystems::History`. History then snapshots the generic token. This crate
does not mutate charge.
