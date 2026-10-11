# Capture artifacts

CSV / JSONL / summary text from [`tracy-flight.sh`](../tracy-flight.sh) land
here as `<phase>/<scenario>/r<n>/`. `*.tracy` files stay local (gitignored).
`*-unwrap.csv`, `*-game.log`, and `*-capture.log` are also gitignored.

Do not commit flight data into this tree — keep captures local to the GPU
machine that ran them.
