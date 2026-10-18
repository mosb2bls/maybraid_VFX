# CI Actions

Optional extra GitHub jobs, opted into from the commit message with a
`ci-action::<name>` token. Workflows already do a substring check on
`github.event.head_commit.message`. There is no extra parser.

`git commit-std` commits the whole `commits/{branch}/{head}` file (`-F`), so a
token written there is what CI sees.

## Commit-file block

After `## Agent Dialogue`, append:

````markdown
## Actions

```shell
ci-action::package
```
````

One token per line in that `shell` fence. Omit the section when nothing extra
should run. Known names live in the table below; add a row when you add a
token.

## Index

| Token | When it runs extra work | What it does |
|---|---|---|
| `ci-action::package` | Commit message contains the token (also `main`, published releases, and `workflow_dispatch` when that workflow is present) | Unsigned macOS DMG, Windows zip, Linux Steam depot, and general Linux tarball. Artifacts on the Actions run; Release assets only for a published GitHub Release. |

Label cleanup on a non-`main` branch still uses the older
`debug(ci:labels:deletion)` string in [LABELS.md](../LABELS.md), not a
`ci-action::` token.
