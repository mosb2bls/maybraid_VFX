# Local language models

Development GGUF files for [`maybraid-language-mistral`](../../../language/mistral). Runtime inference is offline; do not fetch models from the network during play.

The default CLI path is:

```text
assets/models/qwen3-0.6b-q4.gguf
```

If that file is missing, the loader falls back to the existing 1.7B Q4 asset:

```text
assets/models/qwen3-1.7b-q4.gguf
```

GGUF files are gitignored. Fetch the Unsloth Qwen3 0.6B Q4_K_M weights with:

```bash
./bin/download-qwen3-gguf.sh
```

Override the path with `--model-path` or `MAYBRAID_QWEN_GGUF`.

On macOS the language crates enable Metal so Qwen can run on the GPU. Darwin 25+
(GPUCompiler 32023+) rejects Candle 0.11's `mlx_gemm.metal` (`thread
simdgroup_matrix`), so load falls back to CPU automatically. The workspace pins
[mistral.rs v0.9.4](https://github.com/EricLBuehler/mistral.rs/releases/tag/v0.9.4)
from git because crates.io stopped at 0.8.1; that release still ships the
unpatched kernels. Force CPU anywhere with `--force-cpu` or
`MAYBRAID_LANGUAGE_FORCE_CPU=1`. Try Metal anyway with
`MAYBRAID_LANGUAGE_FORCE_METAL=1` (load fails with a Metal library error until
Candle qualifies those `simdgroup_matrix` locals). Linux CUDA is `--features cuda`.

`maybraid-language` enables mistral.rs load and throughput logs. `RUST_LOG`
overrides the default `info` filter; `MISTRALRS_DEBUG=1` raises it to `debug`.
Respond generation is capped at 128 tokens so it cannot fill the Qwen3
40960-token context. Qwen emits English only; UDPipe owns the parse.

Qwen3 is used under its own license from Alibaba. This project does not embed the weights.
