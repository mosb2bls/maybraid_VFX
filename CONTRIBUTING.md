# Contributing

| Task | Description |
|------|-------------|
| [Upcoming Events](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Aevent%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `event` issues with planned completion dates. |
| [Release Candidates](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Arelease-candidate%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | Feature-complete versions linked to events. |
| [Features & Bugs](https://github.com/ramate-io/maybraid/issues?q=is%3Aissue%20state%3Aopen%20label%3Afeature%2Cbug%2Cproposal%20label%3Apriority%3Ahigh%2Cpriority%3Aurgent) | High-priority `feature` and `bug` issues. |

Each issue should have a priority. All Releases Candidates should eventually link up to Events. All Features and Bugs should eventually link up to Release Candidates. 

Proposals issue labels are generally used to mark requests for research.

All issues should tag the following projects:

- `Ramate`: https://github.com/orgs/ramate-io/projects/2
- `Maybraid`: https://github.com/orgs/ramate-io/projects/17

## Events

Events should be titled as follows:

```
Event: <name of the event>
```

Events should be formatted in Markdown as follows:

```
# Summary
- **Target date:** <insert date>

Two to three line description of the event.

- Bulleted
- Line Item
- Callouts

Any other description as the writer sees fit...
```

## Release Candidates

Release Candidates should be titled as follows:

```
Release Candidate: <name of the release candidate>
```

Release Candidates should be formatted as follows:

```
# Summary

This Release Candidate is intended to enable the following events:

- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)
- [Link to Event Issue](https://github.com/ramate-io/gwrdfa/issues/4)

Two to three line description of the release candidate. 

- Bulleted
- Line Item
- Callout

Any other description as the writer sees fit...
```

## Features & Bugs

A Feature or Bug should be titled as follows:

```
<One sentence description of what the bug or feature does.>
```

Features or Bugs should be formatted as follows:

```
# Summary

<One sentence description of what the bug or feature does.>

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

## Proposals 

A Proposal should be titled as follows:

```
<One sentence description of the problem which should be proposed against.>
```

Proposals should be formatted as follows:

```
# Summary

<One sentence description of the problem which should be proposed against.>.

- Bulleted 
- Line Item
- Callouts

Any other description as the writer sees fit.
```

Proposals will generally be satisfied by additions to repository documentation or inclusion of a written proposal in another knowledge base.

Common external knowledge bases include:

- [OAC](https://github.com/ramate-io/oac)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Ramate](https://github.com/ramate-io/ramate)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.
- [Robles](https://github.com/ramate-io/robles)
    - Proposal issue should be followed up with desiderata in the repository.
    - Proposal issue would be closed with a corresponding spec.

> [!TIP]
> **Preferred:** use [`bin/publish-gh-issue.sh`](bin/publish-gh-issue.sh) with a small JSON manifest next to your Markdown body—see [`bin/publish-gh-issue.md`](bin/publish-gh-issue.md). That covers **`gh issue create`**, **sub-issue parent** (UI relationship—not the same as linking in the body), and **`Ramate` / `Maybraid` org projects** (defaults: project **2** and **17** on `ramate-io`). `gh issue create -p …` often fails for org projects; the script uses `gh project item-add` instead (needs `gh auth refresh -s project -s read:project`).
>
> ```bash
> ./bin/publish-gh-issue.sh issues/your-scope/issue.json
> ```
>
> **Manual sequence** (same end state as the script):
>
> ```bash
> # 1) Create issue (repo, title, body, labels—maybraid unless you override in JSON)
> gh issue create -R ramate-io/maybraid --title '…' --body-file body.md -l feature -l priority:medium
>
> # 2) Link new issue as sub-issue of the parent (parent = RC or Jersey epic, child = new)
> PARENT_NODE=$(gh api repos/ramate-io/maybraid/issues/<PARENT#> -q .node_id)
> CHILD_NODE=$(gh api repos/ramate-io/maybraid/issues/<NEW#> -q .node_id)
> gh api graphql -f query='mutation($i: ID!, $s: ID!) { addSubIssue(input: {issueId: $i, subIssueId: $s}) { issue { number } subIssue { number } } }' -f i="$PARENT_NODE" -f s="$CHILD_NODE"
>
> # 3) Org projects (numbers match “All issues should tag…” above: Ramate 2, Maybraid 17)
> gh project item-add 2 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> gh project item-add 17 --owner ramate-io --url https://github.com/ramate-io/maybraid/issues/<NEW#>
> ```

## Rust Style

Prefer **methods on types** over free-floating functions. If logic is about constructing, querying, or transforming a value, put it on the relevant struct/enum (`Foo::build…`, `Foo::from_…`, `foo.sample…`) rather than a module-level helper that takes that type as its first argument. Free functions are fine for true utilities with no natural owner (e.g. tiny pure math), trait impls, and macros—but default to attaching behavior to a type so call sites stay discoverable and naming stays coherent.

Reserve generation **"cell"** terminology for LOD / cellular generation (`OriginCell`, `GenerationScheme`, cell layouts). Bounded rectangles used by shared procedural walks should use names like `Bounds2`, not `*Cell`.

### Modules: never `mod.rs` (hard rule)

> [!CAUTION]
> **Never use `mod.rs`.** Not for new modules, not when splitting a file, not “temporarily.” Every `foo/mod.rs` in this repository is a style bug. The only accepted layout is `foo.rs` next to a `foo/` directory of children.

Agents re-introduce `mod.rs` often — stop and rewrite before continuing. Same rule in nested crates (e.g. Richmond buildings): `monotower.rs` + `monotower/les_halles.rs`, never `monotowers/mod.rs`.

| Wrong | Right |
|-------|--------|
| `foo/mod.rs` + `foo/bar.rs` | `foo.rs` + `foo/bar.rs` |
| `monotowers/mod.rs` + `monotowers/mixed_use_les_halles.rs` | `monotower.rs` + `monotower/les_halles.rs` |
| `usage_plan/mod.rs` + `usage_plan/livable.rs` | `usage_plan.rs` + `usage_plan/livable.rs` |

Rules:

1. The module root is always a **sibling `.rs` file** next to its subdirectory (`foo.rs` declares `pub mod bar;` and lives beside `foo/`).
2. Submodules live **inside** that directory (`foo/bar.rs`, `foo/baz.rs`) — never as `foo/mod.rs`.
3. If you are about to write `mod.rs`, stop and rename: move the contents to `foo.rs` and keep children under `foo/`.
4. Reviewers / agents: treat any new `**/mod.rs` as a **merge-blocking** style violation.

Prefer **`use crate::…`** / concrete paths instead of stitching **`super::`** chains unless there is a compelling reason (e.g., deliberate coupling to an immediate parent in a macro-heavy submodule).

## Rust Tests

Do **not** use [`.unwrap()`](https://doc.rust-lang.org/std/option/enum.Option.html#method.unwrap), [`.expect(...)`](https://doc.rust-lang.org/std/option/enum.Option.html#method.expect), or [`panic!(...)`](https://doc.rust-lang.org/std/macro.panic.html) in test bodies—those snippets are often copied into production code and keep failing habits.

Prefer **`Result`** propagation instead: write helpers that return something like **`anyhow::Result`** (or your crate’s error type), use **`?`**, and declare **`#[test] fn case() -> anyhow::Result<()>`**, so harness failures surface structured errors. [`assert!`](https://doc.rust-lang.org/std/macro.assert.html) / [`assert_eq!`](https://doc.rust-lang.org/std/macro.assert_eq.html) remain appropriate for expectations.

## Packaging and CI

When working on game packaging or scripts in `packaging/`, include the **`ci-action::package`** marker in your commit message to trigger the [Package workflow](../.github/workflows/package.yml) on your PR branch. Without it, Package only runs on `main`, published releases, and manual `workflow_dispatch`.

The [Package workflow](../.github/workflows/package.yml) builds unsigned macOS DMG, Windows zip, Linux Steam depot, and general Linux tarball artifacts. It runs on `main` and published releases automatically, but for feature branches you must opt in with the commit message marker. Local reproduction commands are in [packaging/README.md](packaging/README.md).

To include the marker, add an `## Actions` section after `## Agent Dialogue` in your commit message file:

````markdown
## Actions

```shell
ci-action::package
```
````

When you commit with `git commit-std`, the full commit message file (including the Actions section) becomes the commit message, and GitHub's workflow checks will see the token. See [ci-actions/README.md](../.github/workflows/ci-actions/README.md) for the index of available markers.

**When to use `ci-action::package`:**
- Changes to `packaging/scripts/` or packaging configuration
- Modifications to build flags or linker settings that affect release binaries
- Updates to the Package workflow or toolchain pins that could impact package outputs
- Any change where you need to verify the packaged artifacts work correctly

Without the marker, your CI checks will pass but no packages will be built, and packaging regressions may only surface later on `main`.

## Traces Releases

Profiling captures (Tracy `.tracy` files and Xcode Instruments `.trace` bundles) are published as **GitHub Release assets**, never committed to git. Each capture asset has an identically named companion `.md` with findings for that file.

### Release identity

One Traces release per workspace version. Read `X.Y.Z` from `[workspace.package] version` in the root `Cargo.toml`.

| | |
|---|---|
| Title | `vX.Y.Z Traces` |
| Tag | `vX.Y.Z-traces` |

Later captures for the same version are **appended** as assets on that release. Do not open a second Traces release for the same version.

The release must **not** be Latest, a pre-release, or a draft:

```bash
gh release create vX.Y.Z-traces \
  --title "vX.Y.Z Traces" \
  --latest=false \
  --notes-file notes.md
```

Do not pass `--prerelease` or `--draft`. Release notes are an index table: file stem, scenario/setting, commit, date, device, one-line finding.

### Packaging guard

[`.github/workflows/package.yml`](.github/workflows/package.yml) runs on `release: published`. Its `publish-release` job builds the game and uploads DMG/zip/tarball into the release. That must not happen for Traces releases. Every job in that workflow skips when:

```
github.event_name == 'release' && endsWith(github.event.release.tag_name, '-traces')
```

**Release events use the workflow file at the tagged commit.** A Traces tag must point at a commit that already contains this guard. Until that change is on `main`, target the branch commit that added it; afterwards, any `main` commit is fine.

### Asset naming

```
<commit7>_<YYYYMMDD>_<device>_<setting>.<ext>
```

Example: `1f79999_20261011_m4pro-macmini_crowd-16-ab-after-r1.tracy`

| Field | Meaning |
|---|---|
| `commit7` | 7-character commit of the **source the binary was built from** |
| `YYYYMMDD` | Capture date (UTC) |
| `device` | Short slug, e.g. `m4pro-macmini` |
| `setting` | Kebab-case scenario + variant + repeat, e.g. `crowd-16-ab-after-r1`, `crowd-0-gpu-metal-r1` |
| `ext` | `tracy` for Tracy; `trace.zip` for zipped Instruments bundles |

Zip Instruments bundles with `ditto -c -k --keepParent X.trace X.trace.zip`. Each asset has a companion with the **identical stem** and `.md` (e.g. `1f79999_20261011_m4pro-macmini_crowd-16-ab-after-r1.md`).

Hard limits: **2 GB per asset**. Exclude incomplete, failed, or `Monitor removed` runs.

### Companion `.md` template

```markdown
# <asset file name>

- **Tool:** Tracy 0.13.1 `tracy-capture`  (or Xcode Instruments Metal System Trace)
- **Commit:** [`<full hash>`](https://github.com/ramate-io/maybraid/commit/<full hash>)
- **Date:** YYYY-MM-DD (UTC)
- **Device:** Apple M4 Pro Mac mini, 48 GB, display 1920×1080 @ 60 Hz
- **Build:** release, `--features bevy/trace_tracy`
- **Env:** `MAYBRAID_VSYNC=off`
- **Command:** <script or attach command>
- **Setting:** <scenario, actor count, repeat>

## Summary metrics

| window | frames | main app | render app | present wait | GPU ms |
|---|---|---|---|---|---|
| … | … | … | … | … | … |

## Findings

- …
- …

## Related

- [PR](…) · [issue](…) · [gist](…)
- Sibling: `<partner stem>`

## How to open

Download the asset first. Open `.tracy` in Tracy GUI 0.13.x, or the [web viewer](https://topolarity.github.io/trace-viewer/) (release-asset URLs may fail CORS — download, then open the local file). Unzip `.trace.zip` and open the bundle in Instruments.
```

Findings should be 3–8 bullets about **this file**, including caveats (throttled regime, lost encoder labels, GPU clock state).

### Latest Traces release

Traces releases are never marked Latest, so "the latest Traces release" is the `*-traces` tag with the highest `X.Y.Z`:

```bash
gh release list -R ramate-io/maybraid -L 100 --json tagName -q '.[].tagName' \
  | grep -- '-traces$' | sort -V | tail -1
```

If the workspace version has moved past it, create the new version's release first (see [Release identity](#release-identity)).

### Gameplay scripts as assets

Agent scripts ([`maybraid/agent`](maybraid/agent/README.md)) can be published on a Traces release, so capture runs can be reproduced and later scripts can build on earlier ones. They use the same naming with `script` in the device slot:

```
<commit7>_<YYYYMMDD>_script_<setting>.jsonl
<commit7>_<YYYYMMDD>_script_<setting>.md
```

- `commit7` is the commit whose script API the script was written and parse-checked against.
- The measure window is a `<setting>-warmup` / `<setting>` pair of `scenario` marks, followed by `quit`. The flight scripts read the last non-`-warmup` start mark.
- Choose the start location with a leading `{"cmd":"start","xz":[X,Z]}` rather than relying on `--start-at`.
- The companion `.md` states the **script API version** ([agent README](maybraid/agent/README.md#script-api-version)). It also covers start location, route, what the script exercises (streaming, locomotion, crowds, combat, …), which zones or passes it should stress, and which earlier scripts it extends.

Check that a script parses before publishing it: run `profiling/tracy-flight.sh path/to/script.jsonl check 1`, or `cargo test -p maybraid-agent` with the script copied into `maybraid/agent/scripts/` locally.

### Menu playtest scripts

Menu scripts are regression tests, not Traces captures. They live in-repo as
[`maybraid/agent/scripts/menu-*.jsonl`](maybraid/agent/scripts/) (script API 6+)
and each file's header comment says what it covers.

They need menu control and observation that world gameplay scripts do not:
`confirm` / `back` / `start` aliases, same-frame `press.buttons` chords,
`wait_for` / `expect` / `mash`, and `screen` / `modal` / `focus` / `flow` /
`invariant` events. A script with no leading `start` and no `--start-at` boots
to Home and becomes ready when the first menu screen is up (`ready` detail
`menu`). See the [agent README](maybraid/agent/README.md#script-api-version).

Write a new script by copying a `menu-*.jsonl` neighbor. Put a `wait` of a
couple of frames between pad presses so two `press` commands do not land on
the same tick. Prefer `wait_for` / `expect` over long `wait`s. Put `mash`
seeds in the filename (`menu-mash-home-42.jsonl`) so a failure names the
reproduction seed.

Check that a script parses with `cargo test -p maybraid-agent`. Run the suite
against throwaway saves:

```bash
cargo build -p maybraid
bin/menu-playtest.sh
bin/menu-playtest.sh --seeds 3          # repeat each mash script with N seeds
bin/menu-playtest.sh menu-create-back-mid-spin
```

The runner sets `MAYBRAID_SAVES` / `MAYBRAID_LOGS` to a temp dir, seeds a
fixture roster (`empty`, `one`, or `several`), and uses a small window
(`MAYBRAID_RESOLUTION`, default `960x540`). A run fails on a non-zero exit,
a `panic.log`, a missing `quit` event, any `failed` command, or any
`invariant` event. It prints the failing mash seed.

Do not commit screenshots or playtest logs. Capture shots only with the
in-engine `{"cmd":"console","line":"screenshot PATH"}` command (GPU readback).
A crash found by a menu script is a `bug` (plus `trace` when it came from a
Traces asset), linking the script and seed.

CPU captures come from `profiling/tracy-flight.sh <script.jsonl> <phase> <repeat>`, and GPU captures from `profiling/metal-flight.sh` (Metal System Trace; uploads `*.trace.zip`). Both accept a downloaded script path. See [`profiling/README.md`](profiling/README.md).

### The `trace` label

Issues, discussions, and PRs that come from Traces releases carry the `trace` label. That covers hitch, CPU, or GPU findings; crashes found in captures (`trace` + `bug`); and optimizations that cite a capture. Link the release assets they discuss. Optimization PRs post before/after captures to the release under the same naming.

### Daily trace routines

Scheduled agents keep the Traces release current. Times are CET; each routine is independent and skips quietly when it has nothing to do.

| Time | Routine | Output |
|---|---|---|
| 07:00 | Bug fixes | PRs fixing open `trace` + `bug` issues |
| 08:00 | Script writer | One new `script_<setting>.jsonl` + `.md` on the latest Traces release |
| 09:00 | Trace capturer | CPU `.tracy` and GPU `.trace.zip` captures (+ `.md`) of interesting scripts — only when the `maybraid-gpu` machine is available |
| 11:00 | Hitch finder | `trace` issue/discussion on frame hitches; `trace` + `bug` issue per crash |
| 11:00 | CPU performance | `trace` issue/discussion on steady-state CPU zones |
| 11:00 | GPU performance | `trace` issue/discussion on GPU passes |
| 16:00 | Live optimizer | One well-scoped `trace` PR addressing captured findings, with before/after captures when `maybraid-gpu` is available |

Rules for every routine:

1. Read the latest Traces release's assets and companion `.md` files first, and do not repeat an existing script, capture, or finding. Search open `trace` issues and discussions before filing.
2. Capture only on `maybraid-gpu`, one game at a time. The flight scripts refuse to start when `maybraid`, `tracy-capture`, or `xctrace` is running. Treat that as "machine unavailable" and skip.
3. Asset names and companion `.md` files follow this section. Upload with `gh release upload vX.Y.Z-traces <files> -R ramate-io/maybraid`, never `--clobber` someone else's asset.
4. Findings cite asset names and numbers from them (window means, p95, hitch frames). Speculation is labeled as such.

## Spotting and combat targeting

Keep semantic eligibility, visual knowledge, combat membership, and weapon
opportunity as separate layers:

1. [`SpotSubject`](maybraid/intelligence/spotting/lib/src/subject.rs) identifies
   something that may be observed. Semantic interest layers describe what it
   is; they are not physics collision layers.
2. [`SpottingUser`](maybraid/intelligence/spotting/lib/src/user.rs) owns
   directives, explicit subject hints, and remembered visual contacts.
   Broadphase discovery and explicit hints are complementary candidate sources:
   merge both into the same bounded visibility executor.
3. A candidate becomes a [`SpottedContact`](maybraid/intelligence/spotting/lib/src/contact.rs)
   only after a successful visibility probe. Being a known enemy, objective, or
   recent attacker is not equivalent to currently seeing it.
4. [`CombatTargeting`](maybraid/intelligence/combat/targeting/src/targeting.rs)
   owns semantic target membership, contact snapshots, factor algebra, and
   ranking. Use independent [`TargetSource`](maybraid/intelligence/combat/targeting/src/source.rs)
   bits so enemyship, objectives, spotting, and received fire can add or remove
   their own reasons without overwriting one another.
5. Weapon intelligence consumes actionable ranked contacts and contributes
   weapon-specific opportunity. Eye-level spotting and muzzle-level trajectory
   validation are distinct visibility tasks and should retain separate caches
   and cadences. Whether the combatant may actually raise the trigger is a
   per-entity [`FirearmEngagement`](maybraid/intelligence/combat/firearm/src/engagement.rs)
   grant (`Hold`, `ReturnFire`, or `WeaponsFree`), not a global fire lock and
   not a substitute for threat classification.
6. [`threat-intelligence-damage`](maybraid/intelligence/threat/damage) maps
   [`DamageApplied`](maybraid/damage/src/lib.rs) onto decaying individual
   antagonism and a directed `RECEIVED_DAMAGE` observation. That is typed
   knowledge for the victim; it does not fabricate a sighting.
7. Pack share writes `SHARED` knowledge, not sightings. [`MobKnowledge`](maybraid/intelligence/mob/src/share.rs)
   unions first-hand threat ids and shareable combat-target membership on the
   host, then fans missing entries to live mates as `ThreatSource::SHARED` /
   `TargetSource::SHARED`. Recipients still need their own visibility probe
   before `SpottedContact` or combat contact memory exists.

An active target may intentionally have no current contact memory. Consumers
must therefore select the highest-ranked target with the knowledge they require,
not blindly take rank zero and fail when its contact is absent. Likewise,
eligibility filters must describe the **target**, not the observer: filtering
firearm trajectory subjects with `Without<FirearmIntelligence>` excludes every
armed NPC from NPC-to-NPC combat. Exclude self explicitly by entity identity.

Gameplay events should write typed knowledge rather than secretly bypassing the
pipeline. For example, a session may flip combatants from `Hold` to
`WeaponsFree`, while received fire or applied damage adds a source-owned
observation or decaying antagonism; neither event should fabricate a successful
sighting. Avoid reading live transforms as remembered knowledge. Resolve live
transforms only inside the visibility executor that is actively checking them,
then store the resulting observation snapshot.

Applications own budgets and cadence. Keep reusable spotting and targeting
plugins cadence-neutral; configure discovery, respotting, ranking, movement,
and muzzle validation independently in the application. Add a vertical
integration test that exercises `candidate source → visibility → contact →
target rank → weapon trajectory`, including peer combatants that carry the same
intelligence components as the observer.

Civilian evasion follows the same knowledge split. [`EvasionIntelligenceUser`](maybraid/intelligence/evasion/src/user.rs)
is assailant memory and exclusive hide | flee routing, not a movement mixer.
[`fleeing-intelligence`](maybraid/intelligence/fleeing) and
[`hiding-intelligence`](maybraid/intelligence/hiding) consume that signal and
write [`MovementObjective`](maybraid/intelligence/movement/lib/src/objective.rs).
A heard shot is `RECEIVED_FIRE`, never a fake sighting. Hide occupancy queries
must identify other subjects by entity, not `Without<EvasionIntelligenceUser>`.
Combat spotting looks for [`CHARACTER`](maybraid/intelligence/spotting/lib/src/layers.rs);
civilians use a distinct `CIVILIAN` interest layer so they occupy hide pockets
without entering firearm target memory.

Long-range walking is [`routing-intelligence`](maybraid/intelligence/routing).
Band segment lengths belong to the mover. Routing commits a corridor, then
writes a nearby [`Reach`](maybraid/intelligence/movement/lib/src/objective.rs);
it does not replace local movement planning.

Following a live entity is [`tether-intelligence`](maybraid/intelligence/tether).
Install [`TetherIntelligenceUser`](maybraid/intelligence/tether/src/user.rs) for
the duty and set `enabled` when it may write; keep
[`TetherMemory`](maybraid/intelligence/tether/src/memory.rs) when the brain is
popped. Close remaining work goes to movement; far remaining work goes to
routing. Do not add/remove the user every time `satisfied` flips.

## Performance diagnostics (Tracy first)

Profile LOD and playground hitches with **Tracy**, not in-app `eprintln` / `info!` counters. Bevy already emits `system` / `system_commands` / `par_for_each` zones when built with `trace`. Export a single-frame CSV (“limited to view”) or use `tracy-csvexport` when you need to share a capture.

Do **not** add hitch loggers to `lod`, playgrounds, or `ApplyDeferred` paths. Those scans (`Added<ChildOf>`, `Added<SceneRefRoot>`, …) showed up as hundreds of microseconds on the frames they were meant to explain.

If you need text logs again (composition of a drain wave, command-apply ms, FPS):

1. Put them in a **dedicated diagnostic crate** (for example `maybraid/lod/diagnostics`), not in `lod`’s fulfill systems.
2. Gate the crate and every subscriber behind a **Cargo feature** (compiler flag), default-off. An env var is fine as a secondary switch once the feature is on.
3. Keep playgrounds free of `PlaygroundDiag` / `system_commands` tracing layers. Wire the crate from `main` only when the feature is enabled.

Copies of Tracy CSVs and hitch logs from the orchard work (`frame_*.csv`, `*tracy-zones*.csv`, `vast-orchard-hitch.log`) were removed from the tree. Recover the same kind of capture from **git history** on this branch if you need a worked example of the problems (drain apply, cull DFS, per-type produce, visibility reveal).

## Migrating a grove to the orchard (flattened) approach

Orchard High/Medium plants are **posed kit content**, not a nest of per-stick / per-ball [`LodSceneHost`](maybraid/lod/lib/src/scene/host.rs)s. Forest tiles go further: kits live under the **grove** host (`ChicoGroveHost` / leftover grove `LodScene`), so hide/show is one apply per tile. Isolated `/show` trees still keep one Gimme-indexed host per plant. Shared stick/ball [`SceneRef`](maybraid/scene-ref)s; no fine-phase refresh per kit node.

Canonical example: [`maybraid/world/layers/vegetation/groves/src/orchard.rs`](maybraid/world/layers/vegetation/groves/src/orchard.rs) (`nest_plant_chunks`) plus helpers in [`grove/vc_compose.rs`](maybraid/world/layers/vegetation/groves/src/grove/vc_compose.rs).

1. **Compose with `nest_flattened_plant_chunk`**, not the unused nested-host helpers in [`placed_host.rs`](maybraid/world/layers/vegetation/groves/src/grove/placed_host.rs). Those wrap [`ComponentsOnly`](maybraid/world/layers/vegetation/components/src/lib.rs)`<PlacedVegetation<T>>` and spawn nested [`FoliageNode`](maybraid/world/layers/vegetation/components/src/foliage/node.rs) / [`StickNode`](maybraid/world/layers/vegetation/components/src/sticks/node.rs) LOD hosts. `nest_flattened_plant_chunk` stamps `FlattenedComponentsOnly<PlacedVegetation<T>>` and emits posed kits under the **grove** host — no per-tree [`LodSceneHost`](maybraid/lod/lib/src/scene/host.rs).
2. **Share the plant type with `Arc<T>`** when `T` is large (Storybook trees). Orchard stores `Arc<StorybookTree>` so begin/drain does not clone geometry per chunk. Register **that** wrapper in the playground:
   `flattened_plant_host!(app, YourTree);`
   in [`view.rs`](maybraid/world/layers/vegetation/chico/src/view.rs). Isolated leftover plant hosts use the same family.
3. **Lazy `SceneChunk` for the plant list.** Use [`lazy_flattened_plant_chunks`](maybraid/world/layers/vegetation/groves/src/grove/vc_compose.rs) (kit weight × plant count) that yields `nest_flattened_plant_chunk` per plant (see Orchard `nest_plant_chunks`). Begin must not box every `scene_with_level` up front.
4. **Leave Low / UltraLow as canopy proxies** (`canopy_proxy_site`, `ULTRA_LOW_CANOPY_BIN_METERS`). Flattening is for High/Medium posed plants (palm-only groves also pose through Low).
5. **Charge kit weight.** Flattened kits use [`FLATTENED_KIT_CHUNK_WEIGHT`](maybraid/world/layers/vegetation/components/src/lib.rs) so drain does not admit a full SceneRef / `WorldAssetRoot` wave in one frame.
6. **Do not add a second produce plugin per region channel.** `GimmeLodSceneRefreshPlugin<T, M, F>` can still be added for bullseye and spotlight; fill/emit are registered once per `T`. Cull fill is the same pattern: one untyped host-hit query (`LodCullProduceCache`), then per-`T` enqueue (`GimmeLodSceneCullPlugin`).

7. **Quantize + merge kits on the plant.** Step-by-step for other constructions: [`sbs-trees` CONTRIBUTING](maybraid/world/layers/vegetation/sbs-trees/CONTRIBUTING.md) (tree / tuft `unit_from_num` + collection merge) and [`vegetation-groves` CONTRIBUTING](maybraid/world/layers/vegetation/groves/CONTRIBUTING.md) (`tree_variants` / flatten). Orchard uses [`StorybookTree::unit_from_num`](maybraid/world/layers/vegetation/sbs-trees/src/storybook_tree.rs) (`tree_variants`, default 100). Emission folds sticks and cheap balls into [`MultiSceneMerge`](maybraid/scene-ref) collections. Merge packs kit-local positions into vertex color so [`LeafMaterial`](maybraid/world/layers/vegetation/shaders/src/leaf_material.wgsl) breakup still works. World size stays on the plant [`Placement`](maybraid/world/layers/vegetation/components/src/placed.rs) scale.

A grove that still uses the nested-host helpers in [`placed_host.rs`](maybraid/world/layers/vegetation/groves/src/grove/placed_host.rs) will pay per-node hosts, per-type produce, and a larger visibility/transform wave. Flatten when the plants are kit instances (shared meshes) and fine-phase LOD on each stick/ball is not required.