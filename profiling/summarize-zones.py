#!/usr/bin/env python3
"""Summarize Tracy zone CSVs. Names are unquoted and may contain commas."""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path


INTERESTING = (
	"Update",
	"Frame",
	"fill_lod",
	"fulfill",
	"drain_lod",
	"visibility_propagate",
	"write_binned",
	"check_visibility",
	"sync_stick",
	"queue_terrain",
	"begin_chunk",
	"render_system",
	"extract",
	"PhysicsSchedule",
	"reindex_moved",
	"chico_grove",
	"padded_terrain",
	"Present",
	"prepare_view",
	"queue_material",
	"shadow",
)


def parse_args() -> argparse.Namespace:
	parser = argparse.ArgumentParser()
	parser.add_argument("--zones", required=True, type=Path)
	parser.add_argument("--events", type=Path)
	parser.add_argument("--unwrap", type=Path)
	parser.add_argument("--scenario", default="")
	parser.add_argument("--out", type=Path)
	parser.add_argument("--top", type=int, default=25)
	return parser.parse_args()


def measure_window(events_path: Path | None, scenario: str) -> tuple[float | None, float | None]:
	if events_path is None or not events_path.is_file() or not scenario:
		return None, None
	start = None
	end = None
	for raw in events_path.read_text().splitlines():
		line = raw.strip()
		if not line:
			continue
		try:
			event = json.loads(line)
		except json.JSONDecodeError:
			continue
		if event.get("kind") != "scenario" or event.get("name") != scenario:
			continue
		if event.get("mark") == "start":
			start = float(event["t"])
		elif event.get("mark") == "end":
			end = float(event["t"])
	return start, end


def split_name_path(line: str) -> tuple[str, str] | None:
	idx = line.find(",/")
	if idx < 0:
		idx = line.find(",\\")
	if idx < 0:
		return None
	return line[:idx], line[idx + 1 :]


def parse_zone_row(line: str) -> dict[str, float | str | int] | None:
	split = split_name_path(line)
	if split is None:
		return None
	name, rest = split
	# src_file,src_line,total_ns,total_perc,counts,mean_ns,min_ns,max_ns,std_ns
	parts = rest.split(",")
	if len(parts) < 9:
		return None
	try:
		return {
			"name": name,
			"src_file": parts[0],
			"count": int(float(parts[4])),
			"total_ns": float(parts[2]),
			"mean_ns": float(parts[5]),
			"min_ns": float(parts[6]),
			"max_ns": float(parts[7]),
		}
	except ValueError:
		return None


def parse_unwrap_row(line: str) -> dict[str, float | str] | None:
	split = split_name_path(line)
	if split is None:
		return None
	name, rest = split
	# src_file,src_line,ns_since_start,exec_time_ns,thread,value
	parts = rest.split(",")
	if len(parts) < 4:
		return None
	try:
		return {
			"name": name,
			"start_ns": float(parts[2]),
			"duration_ns": float(parts[3]),
		}
	except ValueError:
		return None


def ms(ns: float) -> str:
	return f"{ns / 1_000_000:.3f}"


def interesting(name: str) -> bool:
	lower = name.lower()
	return any(token.lower() in lower for token in INTERESTING)


def startup_only(name: str, count: int) -> bool:
	return count <= 2 and (
		name.startswith("plugin ") or name.startswith("asset loading") or "startup" in name.lower()
	)


def load_zones(path: Path) -> list[dict[str, float | str | int]]:
	rows: list[dict[str, float | str | int]] = []
	for raw in path.read_text(errors="replace").splitlines()[1:]:
		row = parse_zone_row(raw)
		if row is not None:
			rows.append(row)
	return rows


def summarize_aggregate(rows: list[dict[str, float | str | int]], top: int) -> list[dict[str, float | str | int]]:
	recurring = [
		row
		for row in rows
		if int(row["count"]) >= 8 and not startup_only(str(row["name"]), int(row["count"]))
	]
	hot = [row for row in recurring if interesting(str(row["name"]))]
	pool = hot or recurring
	pool.sort(key=lambda row: float(row["mean_ns"]) * int(row["count"]), reverse=True)
	return pool[:top]


def summarize_unwrap(
	path: Path,
	start_s: float | None,
	end_s: float | None,
	top: int,
) -> list[tuple[str, int, float, float]]:
	by_name: dict[str, list[float]] = defaultdict(list)
	for raw in path.read_text(errors="replace").splitlines()[1:]:
		row = parse_unwrap_row(raw)
		if row is None:
			continue
		if start_s is not None:
			t_s = float(row["start_ns"]) / 1_000_000_000
			if t_s < start_s:
				continue
			if end_s is not None and t_s > end_s:
				continue
		by_name[str(row["name"])].append(float(row["duration_ns"]))
	ranked: list[tuple[str, int, float, float]] = []
	for name, samples in by_name.items():
		if len(samples) < 8:
			continue
		ranked.append((name, len(samples), statistics.fmean(samples), max(samples)))
	ranked.sort(key=lambda item: item[2] * item[1], reverse=True)
	hot = [item for item in ranked if interesting(item[0])]
	return (hot or ranked)[:top]


def render_agg(title: str, rows: list[dict[str, float | str | int]]) -> str:
	lines = [title, f"{'zone':<72} {'n':>7} {'mean_ms':>9} {'max_ms':>9}", "-" * 100]
	for row in rows:
		name = str(row["name"])
		if len(name) > 72:
			name = name[:69] + "..."
		lines.append(
			f"{name:<72} {int(row['count']):>7} {ms(float(row['mean_ns'])):>9} {ms(float(row['max_ns'])):>9}"
		)
	return "\n".join(lines) + "\n"


def render_unwrap(
	title: str,
	rows: list[tuple[str, int, float, float]],
	window: tuple[float | None, float | None],
) -> str:
	start, end = window
	lines = [title]
	if start is not None or end is not None:
		lines.append(f"window t=[{start}, {end}] s")
	lines.append(f"{'zone':<72} {'n':>7} {'mean_ms':>9} {'max_ms':>9}")
	lines.append("-" * 100)
	for name, count, mean, maximum in rows:
		if len(name) > 72:
			name = name[:69] + "..."
		lines.append(f"{name:<72} {count:>7} {ms(mean):>9} {ms(maximum):>9}")
	return "\n".join(lines) + "\n"


def main() -> int:
	args = parse_args()
	window = measure_window(args.events, args.scenario)
	chunks: list[str] = []
	if args.zones.is_file():
		rows = load_zones(args.zones)
		chunks.append(render_agg(f"aggregate recurring zones ({args.zones.name})", summarize_aggregate(rows, args.top)))
	if args.unwrap and args.unwrap.is_file():
		chunks.append(
			render_unwrap(
				f"unwrapped ({args.unwrap.name})",
				summarize_unwrap(args.unwrap, window[0], window[1], args.top),
				window,
			)
		)
	if args.events and args.events.is_file():
		chunks.append(f"events measure={args.scenario or '(none)'} window={window}\n")
	text = "\n".join(chunks) if chunks else "no input\n"
	if args.out:
		args.out.write_text(text)
	sys.stdout.write(text)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
