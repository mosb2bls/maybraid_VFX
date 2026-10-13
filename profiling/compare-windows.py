#!/usr/bin/env python3
"""Compare measure-window stats across phases, scenarios, and repeats.

    python3 profiling/compare-windows.py --phases baseline after \
        --scenarios crowd-0 crowd-4 crowd-16 --zone sync_threat_registry ...

Reads the `*-window.json` files `tracy-flight.sh` writes under
`profiling/captures/<phase>/<scenario>/r*/` and prints Markdown tables of
the mean (and min..max over repeats) amortized ms/frame. Actor counts come
from the last `actors` event inside each run's measure window.
"""

from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "captures"
SUMMARY_KEYS = (
	("frame_ms", "frame"),
	("main_app_ms", "main app"),
	("main_app_p95_ms", "main app p95"),
	("render_app_ms", "render app"),
	("present_wait_ms", "present wait"),
)


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser()
	parser.add_argument("--root", type=Path, default=ROOT)
	parser.add_argument("--phases", nargs="+", required=True)
	parser.add_argument("--scenarios", nargs="+", required=True)
	parser.add_argument(
		"--zone",
		action="append",
		default=[],
		help="substring of a zone name (first match per run); repeatable",
	)
	parser.add_argument("--crate", action="append", default=[], help="crate rollup; repeatable")
	return parser.parse_args()


def runs(root: Path, phase: str, scenario: str) -> list[Path]:
	return sorted((root / phase / scenario).glob("r*/*-window.json"))


def actors(window_json: Path, scenario: str) -> dict | None:
	events = Path(str(window_json).replace("-window.json", "-events.jsonl"))
	if not events.is_file():
		return None
	start = end = None
	census = []
	for raw in events.read_text().splitlines():
		try:
			event = json.loads(raw)
		except json.JSONDecodeError:
			continue
		if event.get("kind") == "scenario" and event.get("name") == scenario:
			if event.get("mark") == "start":
				start = event["t"]
			elif event.get("mark") == "end":
				end = event["t"]
		elif event.get("kind") == "actors":
			census.append(event)
	if start is None:
		return None
	inside = [c for c in census if c["t"] <= (end if end is not None else float("inf"))]
	return inside[-1] if inside else None


def zone_value(data: dict, needle: str) -> float:
	for row in data["zones"]:
		if needle in row["zone"]:
			return row["per_frame_ms"]
	return 0.0


def fmt(values: list[float]) -> str:
	if not values:
		return "–"
	mean = statistics.fmean(values)
	if len(values) == 1:
		return f"{mean:.3f}"
	return f"{mean:.3f} ({min(values):.3f}..{max(values):.3f})"


def main() -> int:
	args = parse_args()
	columns = [(phase, scenario) for scenario in args.scenarios for phase in args.phases]
	loaded: dict[tuple[str, str], list[tuple[Path, dict]]] = {}
	for column in columns:
		loaded[column] = [(path, json.loads(path.read_text())) for path in runs(args.root, *column)]

	header = "| metric | " + " | ".join(f"{s} {p}" for p, s in columns) + " |"
	print(header)
	print("|" + "---|" * (len(columns) + 1))

	def row(label: str, pick) -> None:
		cells = []
		for column in columns:
			cells.append(fmt([pick(path, data) for path, data in loaded[column]]))
		print(f"| {label} | " + " | ".join(cells) + " |")

	print(
		"| runs | "
		+ " | ".join(str(len(loaded[column])) for column in columns)
		+ " |"
	)
	print(
		"| bodies (hosts) | "
		+ " | ".join(
			", ".join(
				f"{c['bodies']} ({c['hosts']})"
				for c in (actors(path, column[1]) for path, _ in loaded[column])
				if c
			)
			or "–"
			for column in columns
		)
		+ " |"
	)
	for key, label in SUMMARY_KEYS:
		row(label + " ms", lambda _p, d, key=key: d["summary"].get(key, 0.0))
	for crate in args.crate:
		row(f"`{crate}` (crate)", lambda _p, d, crate=crate: d["crates"].get(crate, 0.0))
	for needle in args.zone:
		row(f"`{needle}`", lambda _p, d, needle=needle: zone_value(d, needle))
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
