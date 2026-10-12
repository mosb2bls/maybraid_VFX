#!/usr/bin/env python3
"""Measure-window zone stats from a streamed `tracy-csvexport -u` export.

    tracy-csvexport -u capture.tracy | python3 profiling/window-zones.py \
        --messages capture-messages.csv --scenario origin-idle --json out.json

The unwrap export is read from stdin and never written to disk (it is
multiple GB). The window comes from the `agent.scenario <name> Start|End`
Tracy messages, so it is on the same clock as the zones.

Per zone: runs, mean per run, max, and amortized ms per frame (total in
window / frames). Frames are `update` zones that start in the window; the
frame time is window length / frames, so it includes GPU and present waits.
Timer-gated systems only emit a zone when they run, so compare their
amortized cost, not their per-run mean.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

FRAME_ZONE = "update"
MAIN_APP = "main app"
RENDER_APP = "sub app{name=RenderApp}"
# Swapchain acquire. On a windowed macOS surface this blocks on the
# compositor even with `PresentMode::Immediate`, so it pads frames to the
# display rate and hides CPU headroom.
PRESENT_WAIT = 'system{name="bevy_render::view::window::prepare_windows"}'
SYSTEM_RE = re.compile(r'^system\{name="([^"]+)"\}$')


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser()
	parser.add_argument("--messages", required=True, type=Path)
	parser.add_argument("--scenario", required=True)
	parser.add_argument("--json", type=Path)
	parser.add_argument("--out", type=Path)
	parser.add_argument("--top", type=int, default=40)
	parser.add_argument(
		"--match",
		action="append",
		default=[],
		help="extra case-insensitive substring; matching zones are always listed",
	)
	return parser.parse_args()


def scenario_window(messages: Path, scenario: str) -> tuple[int, int]:
	start = end = None
	for raw in messages.read_text(errors="replace").splitlines()[1:]:
		name, _, ns = raw.rpartition(",")
		if f"agent.scenario {scenario} Start" in name and name.rstrip().endswith("Start"):
			start = int(ns)
		elif f"agent.scenario {scenario} End" in name and name.rstrip().endswith("End"):
			end = int(ns)
	if start is None or end is None or end <= start:
		raise SystemExit(f"no complete scenario window for {scenario!r} in {messages}")
	return start, end


def crate_of(zone: str) -> str | None:
	match = SYSTEM_RE.match(zone)
	if not match:
		return None
	path = match.group(1)
	head = re.split(r"[<:]", path, maxsplit=1)[0]
	return head or None


def main() -> int:
	args = parse_args()
	start, end = scenario_window(args.messages, args.scenario)
	count: dict[str, int] = defaultdict(int)
	total: dict[str, int] = defaultdict(int)
	peak: dict[str, int] = defaultdict(int)
	frame_ns: list[int] = []
	main_ns: list[int] = []
	stdin = sys.stdin.buffer
	stdin.readline()
	for raw in stdin:
		parts = raw.rsplit(b",", 6)
		if len(parts) < 7:
			continue
		try:
			t = int(parts[3])
		except ValueError:
			continue
		if t < start or t > end:
			continue
		dur = int(parts[4])
		name = parts[0].decode(errors="replace")
		count[name] += 1
		total[name] += dur
		if dur > peak[name]:
			peak[name] = dur
		if name == FRAME_ZONE:
			frame_ns.append(dur)
		elif name == MAIN_APP:
			main_ns.append(dur)

	frames = max(len(frame_ns), 1)
	window_ms = (end - start) / 1e6
	rows = []
	for name, n in count.items():
		rows.append(
			{
				"zone": name,
				"runs": n,
				"mean_ms": total[name] / n / 1e6,
				"max_ms": peak[name] / 1e6,
				"per_frame_ms": total[name] / frames / 1e6,
			}
		)
	rows.sort(key=lambda row: row["per_frame_ms"], reverse=True)

	crates: dict[str, float] = defaultdict(float)
	for row in rows:
		crate = crate_of(row["zone"])
		if crate:
			crates[crate] += row["per_frame_ms"]
	crate_rows = sorted(crates.items(), key=lambda item: item[1], reverse=True)

	by_zone = {row["zone"]: row for row in rows}
	per_frame = lambda zone: by_zone[zone]["per_frame_ms"] if zone in by_zone else 0.0
	frame_sorted = sorted(frame_ns)
	main_sorted = sorted(main_ns)

	def quantile(values: list[int], q: float) -> float:
		return values[min(len(values) - 1, int(q * len(values)))] / 1e6 if values else 0.0

	p = lambda q: quantile(frame_sorted, q)
	summary = {
		"scenario": args.scenario,
		"window_ms": window_ms,
		"frames": len(frame_ns),
		"frame_ms": window_ms / frames,
		"update_mean_ms": sum(frame_ns) / frames / 1e6,
		"update_p50_ms": p(0.5),
		"update_p95_ms": p(0.95),
		"update_p99_ms": p(0.99),
		"update_max_ms": frame_sorted[-1] / 1e6 if frame_sorted else 0.0,
		"main_app_ms": per_frame(MAIN_APP),
		"main_app_p95_ms": quantile(main_sorted, 0.95),
		"main_app_max_ms": main_sorted[-1] / 1e6 if main_sorted else 0.0,
		"render_app_ms": per_frame(RENDER_APP),
		"present_wait_ms": per_frame(PRESENT_WAIT),
	}

	lines = [
		f"window {args.scenario}: {window_ms / 1000:.1f}s, {len(frame_ns)} frames, "
		f"frame {summary['frame_ms']:.2f} ms, update mean {summary['update_mean_ms']:.2f} "
		f"p50 {summary['update_p50_ms']:.2f} p95 {summary['update_p95_ms']:.2f} "
		f"p99 {summary['update_p99_ms']:.2f} max {summary['update_max_ms']:.2f}",
		f"main app {summary['main_app_ms']:.2f} ms/frame (p95 {summary['main_app_p95_ms']:.2f}, "
		f"max {summary['main_app_max_ms']:.2f}), render app {summary['render_app_ms']:.2f} "
		f"of which present wait {summary['present_wait_ms']:.2f}",
		"",
		f"{'zone':<96} {'runs':>7} {'mean':>8} {'max':>8} {'ms/frm':>8}",
		"-" * 131,
	]
	extra = [token.lower() for token in args.match]
	listed = set()
	for i, row in enumerate(rows):
		keep = i < args.top or any(token in row["zone"].lower() for token in extra)
		if not keep:
			continue
		listed.add(row["zone"])
		zone = row["zone"] if len(row["zone"]) <= 96 else row["zone"][:93] + "..."
		lines.append(
			f"{zone:<96} {row['runs']:>7} {row['mean_ms']:>8.3f} {row['max_ms']:>8.3f} {row['per_frame_ms']:>8.3f}"
		)
	lines += ["", f"{'crate (systems only)':<60} {'ms/frm':>8}", "-" * 69]
	for crate, ms in crate_rows[: args.top]:
		lines.append(f"{crate:<60} {ms:>8.3f}")
	text = "\n".join(lines) + "\n"

	if args.out:
		args.out.write_text(text)
	sys.stdout.write(text)
	if args.json:
		args.json.write_text(
			json.dumps({"summary": summary, "zones": rows, "crates": dict(crate_rows)}, indent=1)
		)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
