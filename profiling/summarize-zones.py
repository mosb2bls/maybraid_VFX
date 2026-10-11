#!/usr/bin/env python3
"""Summarize Tracy zone CSVs, optionally sliced to an agent measure window."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path


WHOLE_FRAME_NAMES = ("Frame", "frame", "GameThread", "Update")
INTERESTING_SUBSTR = (
	"Update",
	"frame",
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
	"Present",
	"extract",
	"PhysicsSchedule",
	"reindex_moved",
	"chico_grove",
	"padded_terrain",
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
		if event.get("kind") != "scenario":
			continue
		if event.get("name") != scenario:
			continue
		if event.get("mark") == "start":
			start = float(event["t"])
		elif event.get("mark") == "end":
			end = float(event["t"])
	return start, end


def ns_from_cell(value: str) -> float | None:
	text = value.strip()
	if not text:
		return None
	try:
		number = float(text)
	except ValueError:
		return None
	# Tracy 0.13 csvexport uses nanoseconds for totals / means.
	return number


def load_aggregate(path: Path) -> list[dict[str, str]]:
	with path.open(newline="") as handle:
		return list(csv.DictReader(handle))


def pick(row: dict[str, str], *names: str) -> str | None:
	lower = {key.lower(): key for key in row}
	for name in names:
		key = lower.get(name.lower())
		if key is not None:
			return row[key]
	return None


def format_ms(ns: float | None) -> str:
	if ns is None:
		return "—"
	return f"{ns / 1_000_000:.3f}"


def interesting(name: str) -> bool:
	return any(token.lower() in name.lower() for token in INTERESTING_SUBSTR)


def summarize_aggregate(rows: list[dict[str, str]], top: int) -> list[tuple[str, int, float, float]]:
	parsed: list[tuple[str, int, float, float]] = []
	for row in rows:
		name = pick(row, "name", "zone", "src_zone")
		if not name:
			continue
		count_raw = pick(row, "count", "counts", "num")
		mean_raw = pick(row, "mean_ns", "mean", "avg")
		total_raw = pick(row, "total_ns", "total", "total_time")
		max_raw = pick(row, "max_ns", "max")
		count = int(float(count_raw)) if count_raw else 0
		mean = ns_from_cell(mean_raw) if mean_raw else None
		total = ns_from_cell(total_raw) if total_raw else None
		maximum = ns_from_cell(max_raw) if max_raw else None
		if mean is None and total is not None and count:
			mean = total / count
		if mean is None:
			continue
		parsed.append((name, count, mean, maximum or mean))
	parsed.sort(key=lambda item: item[2] * max(item[1], 1), reverse=True)
	# Prefer interesting names in the printed table, but keep total-time order
	# for the raw ranking when nothing matches.
	hot = [item for item in parsed if interesting(item[0])]
	return (hot or parsed)[:top]


def summarize_unwrap(
	path: Path,
	start_s: float | None,
	end_s: float | None,
	top: int,
) -> list[tuple[str, int, float, float]]:
	by_name: dict[str, list[float]] = defaultdict(list)
	with path.open(newline="") as handle:
		reader = csv.DictReader(handle)
		for row in reader:
			name = pick(row, "name", "zone")
			if not name:
				continue
			t0 = pick(row, "start", "time_start", "t0", "timestamp")
			t1 = pick(row, "end", "time_end", "t1")
			dur = pick(row, "duration", "time", "dt")
			start = ns_from_cell(t0) if t0 else None
			end = ns_from_cell(t1) if t1 else None
			duration = ns_from_cell(dur) if dur else None
			if duration is None and start is not None and end is not None:
				duration = abs(end - start)
			if duration is None:
				continue
			# Unwrap timestamps are Tracy time (ns). Agent events are game seconds.
			# Align by using the first/last Frame if window is set in seconds —
			# caller should pass None when units cannot be matched. We treat
			# start_s/end_s as game seconds and skip unwrap windowing unless
			# timestamps look like seconds already (< 1e7).
			if start is not None and start_s is not None:
				t_s = start if start < 1e7 else start / 1_000_000_000
				if t_s < start_s:
					continue
				if end_s is not None and t_s > end_s:
					continue
			by_name[name].append(duration)
	ranked: list[tuple[str, int, float, float]] = []
	for name, samples in by_name.items():
		ranked.append((name, len(samples), statistics.fmean(samples), max(samples)))
	ranked.sort(key=lambda item: item[2] * item[1], reverse=True)
	hot = [item for item in ranked if interesting(item[0])]
	return (hot or ranked)[:top]


def render_table(
	title: str,
	rows: list[tuple[str, int, float, float]],
	window: tuple[float | None, float | None],
) -> str:
	lines = [title]
	start, end = window
	if start is not None or end is not None:
		lines.append(f"measure window t=[{start if start is not None else '?'}, {end if end is not None else '?'}] s")
	lines.append(f"{'zone':<56} {'n':>7} {'mean_ms':>10} {'max_ms':>10}")
	lines.append("-" * 86)
	for name, count, mean, maximum in rows:
		lines.append(f"{name:<56} {count:>7} {format_ms(mean):>10} {format_ms(maximum):>10}")
	return "\n".join(lines) + "\n"


def main() -> int:
	args = parse_args()
	window = measure_window(args.events, args.scenario)
	chunks: list[str] = []
	if args.zones.is_file():
		rows = load_aggregate(args.zones)
		if rows:
			chunks.append(
				render_table(
					f"aggregate zones ({args.zones.name})",
					summarize_aggregate(rows, args.top),
					window,
				)
			)
		else:
			chunks.append(f"no rows in {args.zones}\n")
	if args.unwrap and args.unwrap.is_file():
		chunks.append(
			render_table(
				f"unwrapped zones ({args.unwrap.name})",
				summarize_unwrap(args.unwrap, window[0], window[1], args.top),
				window,
			)
		)
	if args.events and args.events.is_file():
		start, end = window
		chunks.append(
			f"events: {args.events.name} measure={args.scenario or '(none)'} "
			f"start={start} end={end}\n"
		)
	text = "\n".join(chunks) if chunks else "no input\n"
	if args.out:
		args.out.write_text(text)
	sys.stdout.write(text)
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
