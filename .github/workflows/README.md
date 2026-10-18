# GitHub Actions

Workflows in this directory run on GitHub-hosted runners. Most compile and
format jobs share [`nix-command.yml`](nix-command.yml): install Nix, enable
flakes, restore [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache)
for `target/` and Cargo registries, then `nix develop --command …` on
`ubuntu-latest`.

`on.push.branches` with an empty list means **every branch**, not only `main`.

| Workflow | Trigger | What it checks or produces |
|---|---|---|
| [Cargo Check](cargo-check.yml) | Every push | `cargo check` in the flake shell |
| [Cargo Test](cargo-test.yml) | Every push | `cargo test --release` in the flake shell |
| [Pre-commit Formatting](pre-commit-formatting.yml) | Every push | Re-runs lib-pre-commit hooks on a throwaway commit and fails if the tree changes. Hooks are CI-only by default; opt in locally with `LIB_PRE_COMMIT_ENABLE=1`. |
| [Labels](labels.yml) | Push to `labels.yml`, or `workflow_dispatch` | Creates/updates the canonical issue labels. Deletes extras only on `main` (or a `debug(ci:labels:deletion)` commit). See [LABELS.md](LABELS.md). |

Reusable [`nix-command.yml`](nix-command.yml) is `workflow_call` only. Other
workflows pass `command`, optional `nix_flake_path` (default `.`), and
`runner`.

Opt-in jobs (`ci-action::<name>` in the commit message) are indexed in
[ci-actions/README.md](ci-actions/README.md).
