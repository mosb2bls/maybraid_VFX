#!/usr/bin/env python3
"""Parse Instruments (xctrace) Metal XML tables into per-frame GPU stats.

Instruments XML interned values with id=/ref= attributes, so this walks rows
with a running id map rather than using ElementTree as a tree.

`metal-flight.sh` exports the tables next to `<stem>` and calls this. Usage:
  metal-passes.py --stem captures/gpu/x/crowd-0/r1/crowd-0-x-r1-STAMP --scenario crowd-0
  metal-passes.py --encoders E.xml --gpu G.xml --pstate P.xml --events E.jsonl \
      --scenario crowd-0 --out-stem /tmp/crowd-0

Writes <stem>-stats.json, <stem>-passes.csv, and <stem>-summary.md.
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import re
import sys
from collections import defaultdict
from xml.etree.ElementTree import iterparse


NS_RE = re.compile(r"^\{.*\}")


def local(tag: str) -> str:
    return NS_RE.sub("", tag)


def store_id(elem, id_map: dict) -> None:
    eid = elem.attrib.get("id")
    if not eid:
        return
    # Prefer fmt (human) then text
    val = elem.attrib.get("fmt")
    if val is None or val == "":
        val = (elem.text or "").strip()
    id_map[eid] = val
    # Also stash numeric text under a parallel key when present
    raw = (elem.text or "").strip()
    if raw:
        id_map[f"{eid}#raw"] = raw
    # Process elements: keep pid so later <process ref="N"/> still resolves.
    if local(elem.tag) == "process":
        for c in list(elem):
            if local(c.tag) == "pid":
                pidv = (c.text or "").strip() or c.attrib.get("fmt")
                if pidv:
                    id_map[f"{eid}#pid"] = pidv


def resolve(elem, id_map: dict, numeric: bool = False):
    ref = elem.attrib.get("ref")
    if ref:
        if numeric:
            raw = id_map.get(f"{ref}#raw")
            if raw is not None:
                return raw
        return id_map.get(ref)
    store_id(elem, id_map)
    if numeric:
        raw = (elem.text or "").strip()
        if raw:
            return raw
    return elem.attrib.get("fmt") or (elem.text or "").strip()


def child_text(elem, id_map: dict) -> str:
    """Flatten a formatted-label / process / etc. to a readable string."""
    parts = []

    def walk(e):
        if e.tag is None:
            return
        t = local(e.tag)
        if t in ("pid",):
            parts.append(resolve(e, id_map, numeric=True) or "")
            return
        if t in ("string", "narrative-text", "metal-object-label", "gpu-channel-name",
                 "gpu-state", "gpu-performance-state", "metal-event-name",
                 "metal-device-name"):
            v = resolve(e, id_map)
            if v:
                parts.append(v)
            return
        if t == "process":
            v = resolve(e, id_map)
            if v:
                parts.append(v)
            # still walk pid
            for c in list(e):
                walk(c)
            return
        if e.text and e.text.strip() and t not in ("row",):
            parts.append(e.text.strip())
        for c in list(e):
            walk(c)

    walk(elem)
    return " ".join(p for p in parts if p).strip()


def parse_schema(schema_elem) -> list[str]:
    mnems = []
    for col in schema_elem:
        if local(col.tag) != "col":
            continue
        mnemonic = None
        for c in col:
            if local(c.tag) == "mnemonic" and c.text:
                mnemonic = c.text.strip()
        mnems.append(mnemonic or f"col{len(mnems)}")
    return mnems


def iter_rows(path: str):
    """Yield (schema_mnemonics, row_element, id_map snapshot-capable map)."""
    id_map: dict = {}
    schema = None
    context = iterparse(path, events=("end", "start"))
    for event, elem in context:
        tag = local(elem.tag)
        if event == "start":
            continue
        if tag == "schema":
            schema = parse_schema(elem)
            elem.clear()
            continue
        if tag != "row":
            # still intern ids on nested completed elems that aren't rows
            store_id(elem, id_map)
            continue
        if schema is None:
            elem.clear()
            continue
        yield schema, elem, id_map
        elem.clear()


def row_values(schema, row, id_map) -> dict:
    kids = [c for c in list(row) if local(c.tag) != "schema"]
    out = {}
    for i, mnem in enumerate(schema):
        if i >= len(kids):
            break
        el = kids[i]
        t = local(el.tag)
        if t == "sentinel":
            out[mnem] = None
            continue
        store_id(el, id_map)
        if t in ("start-time", "duration", "uint64", "uint32", "size-in-bytes",
                 "metal-command-buffer-id", "metal-encoder-id", "connection-uuid64",
                 "gpu-frame-number", "metal-nesting-level", "render-buffer-depth"):
            raw = resolve(el, id_map, numeric=True)
            out[mnem] = raw
            out[f"{mnem}#fmt"] = resolve(el, id_map)
        elif t == "process":
            fmt = resolve(el, id_map)
            pid = None
            ref = el.attrib.get("ref")
            if ref:
                pid = id_map.get(f"{ref}#pid")
            if pid is None:
                for c in el:
                    if local(c.tag) == "pid":
                        pid = resolve(c, id_map, numeric=True)
            out[mnem] = fmt
            out[f"{mnem}#pid"] = pid
        elif t == "thread":
            out[mnem] = resolve(el, id_map)
        else:
            out[mnem] = child_text(el, id_map) or resolve(el, id_map)
    return out


def to_int(v):
    if v is None or v == "":
        return None
    if isinstance(v, int):
        return v
    s = str(v).replace(",", "").strip()
    try:
        if s.lower().startswith("0x"):
            return int(s, 16)
        return int(s)
    except ValueError:
        try:
            return int(float(s))
        except ValueError:
            return None


def ns_to_ms(ns: float) -> float:
    return ns / 1e6


def percentile(xs, p):
    if not xs:
        return None
    ys = sorted(xs)
    if len(ys) == 1:
        return ys[0]
    k = (len(ys) - 1) * p
    f = math.floor(k)
    c = math.ceil(k)
    if f == c:
        return ys[int(k)]
    return ys[f] * (c - k) + ys[c] * (k - f)


def stats(xs):
    if not xs:
        return {"n": 0, "mean": None, "p50": None, "p95": None, "max": None}
    return {
        "n": len(xs),
        "mean": sum(xs) / len(xs),
        "p50": percentile(xs, 0.50),
        "p95": percentile(xs, 0.95),
        "max": max(xs),
    }


def fmt_ms(x):
    if x is None:
        return "—"
    return f"{x:.3f}"


def merge_union(intervals):
    """intervals: list of (start, end) ns. Return total union length."""
    if not intervals:
        return 0
    iv = sorted(intervals)
    total = 0
    cs, ce = iv[0]
    for s, e in iv[1:]:
        if s <= ce:
            if e > ce:
                ce = e
        else:
            total += ce - cs
            cs, ce = s, e
    total += ce - cs
    return total


PASS_BUCKETS = [
    ("prepass", re.compile(r"(early[_ ]prepass|prepass|depth.?pre)", re.I)),
    ("shadows", re.compile(r"shadow", re.I)),
    ("opaque", re.compile(r"opaque", re.I)),
    ("transparent", re.compile(r"transparen|transmissive", re.I)),
    ("post", re.compile(r"tonemap|fog|bloom|fxaa|smaa|taa|upscal|post|deferred.?lighting|cluster", re.I)),
    ("ui", re.compile(r"\bui\b|ui_pass", re.I)),
    ("particles", re.compile(r"hanabi|particle", re.I)),
    ("compute", re.compile(r"preprocess|cull|bin unpack|indirect|slab|sparse buffer|compute", re.I)),
    ("wgpu_internal", re.compile(r"wgpu internal|PendingWrites|Blit Command|Signal|Transit", re.I)),
]


def bucket_for(label: str) -> str:
    if not label:
        return "unlabeled"
    for name, rx in PASS_BUCKETS:
        if rx.search(label):
            return name
    return "other"


def parse_events_window(events_path, scenario):
    """Return (start_s, end_s, census) in game-time seconds, or None."""
    if not events_path or not os.path.exists(events_path):
        return None
    start = end = None
    last_actors = None
    last_t = 0.0
    with open(events_path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                ev = json.loads(line)
            except json.JSONDecodeError:
                continue
            t = float(ev.get("t") or 0)
            last_t = max(last_t, t)
            if ev.get("kind") == "scenario" and ev.get("name") == scenario:
                if ev.get("mark") == "start":
                    start = t
                elif ev.get("mark") == "end":
                    end = t
            if ev.get("kind") == "actors":
                last_actors = ev
    return {
        "start": start,
        "end": end,
        "last_t": last_t,
        "actors": last_actors,
    }


def parse_encoders(path, pid_filter=None):
    """encoder_id -> {label, frame, start_ns, cpu_ns, cmdbuffer_id}"""
    by_enc = {}
    labels_seen = defaultdict(int)
    for schema, row, id_map in iter_rows(path):
        v = row_values(schema, row, id_map)
        pid = to_int(v.get("process#pid"))
        if pid_filter and pid and pid != pid_filter:
            continue
        enc_id = to_int(v.get("encoder-id"))
        label = (v.get("encoder-label") or v.get("cmdbuffer-label") or "").strip()
        # encoder-label may include indexed form; prefer non-indexed
        if label.startswith("[") and "]" in label:
            label = label.split("]", 1)[1].strip()
        frame = to_int(v.get("frame-number"))
        start = to_int(v.get("start"))
        dur = to_int(v.get("duration"))
        if enc_id is None:
            continue
        by_enc[enc_id] = {
            "label": label,
            "frame": frame,
            "start_ns": start,
            "cpu_ns": dur,
            "cmdbuffer_id": to_int(v.get("cmdbuffer-id")),
            "pid": pid,
        }
        if label:
            labels_seen[label] += 1
    return by_enc, labels_seen


def parse_gpu_intervals(path, pid_filter=None):
    rows = []
    for schema, row, id_map in iter_rows(path):
        v = row_values(schema, row, id_map)
        pid = to_int(v.get("process#pid"))
        if pid_filter and pid and pid != pid_filter:
            continue
        start = to_int(v.get("start"))
        dur = to_int(v.get("duration"))
        if start is None or dur is None:
            continue
        rows.append({
            "start_ns": start,
            "dur_ns": dur,
            "end_ns": start + dur,
            "channel": v.get("channel-name") or "",
            "frame": to_int(v.get("frame-number")),
            "label": (v.get("event-label") or "").strip(),
            "obj_label": (v.get("channel-subtitle") or "").strip(),
            "state": v.get("state") or "",
            "cmdbuffer_id": to_int(v.get("cmdbuffer-id")),
            "encoder_id": to_int(v.get("encoder-id")),
            "pid": pid,
            "process": v.get("process") or "",
        })
    return rows


def parse_pstate(path):
    rows = []
    for schema, row, id_map in iter_rows(path):
        v = row_values(schema, row, id_map)
        start = to_int(v.get("start"))
        dur = to_int(v.get("duration"))
        state = v.get("gpu-performance-state") or ""
        if start is None or dur is None:
            continue
        rows.append((start, dur, state))
    return rows


def toc_pid(toc_path):
    if not toc_path or not os.path.exists(toc_path):
        return None
    # <process type="launched" ... pid="13870"
    text = open(toc_path, errors="replace").read()
    m = re.search(r'<process type="launched"[^>]*pid="(\d+)"', text)
    if m:
        return int(m.group(1))
    m = re.search(r'pid="(\d+)"', text)
    return int(m.group(1)) if m else None


def toc_duration(toc_path):
    if not toc_path or not os.path.exists(toc_path):
        return None
    text = open(toc_path, errors="replace").read()
    m = re.search(r"<duration>([0-9.]+)</duration>", text)
    return float(m.group(1)) if m else None


def classify_label(gpu_row, encoders):
    enc = encoders.get(gpu_row["encoder_id"] or -1)
    if enc and enc.get("label"):
        return enc["label"]
    # fall back to object label / event label snippets
    for cand in (gpu_row.get("obj_label"), gpu_row.get("label")):
        if cand and "Command Buffer" not in cand and cand not in ("", "GPU Execution"):
            # strip " (process) 0x..."
            return re.sub(r"\s+\(.*$", "", cand).strip()
    if enc and enc.get("label"):
        return enc["label"]
    return gpu_row.get("obj_label") or "unlabeled"


def analyze(encoders, gpu_rows, pstate, window_ns, pid):
    w0, w1 = window_ns
    gpu = [r for r in gpu_rows if r["end_ns"] >= w0 and r["start_ns"] <= w1]
    # clip to window
    for r in gpu:
        r["cstart"] = max(r["start_ns"], w0)
        r["cend"] = min(r["end_ns"], w1)
        r["cdur"] = max(0, r["cend"] - r["cstart"])

    # map encoder labels
    for r in gpu:
        r["pass"] = classify_label(r, encoders)

    # Frame grouping: prefer Metal frame-number on GPU intervals; fall back
    # to 16.67ms bins of GPU start time. Unique-per-interval frame ids
    # (n_frames ≈ n_gpu_intervals, p50 << 1 ms) are the #1198 discard case —
    # treat them as missing and bin instead.
    numbered = [r["frame"] for r in gpu if r["frame"] is not None]
    unique_numbered = len(set(numbered))
    expected_bins = max(int((w1 - w0) / 16_670_000), 1)
    collapsed = bool(numbered) and unique_numbered > expected_bins * 2
    use_bins = collapsed or not numbered
    by_frame = defaultdict(list)
    unlabeled_frames = 0
    for r in gpu:
        fr = r["frame"]
        if use_bins or fr is None:
            fr = int(r["cstart"] // 16_670_000)
            unlabeled_frames += 1
        by_frame[fr].append(r)

    frame_union = []
    frame_sum = []
    frame_pass = defaultdict(list)  # pass -> list of per-frame union ms
    frame_bucket = defaultdict(list)

    # per-frame pass union
    for fr, rs in by_frame.items():
        union = merge_union([(r["cstart"], r["cend"]) for r in rs])
        sm = sum(r["cdur"] for r in rs)
        frame_union.append(union / 1e6)
        frame_sum.append(sm / 1e6)
        by_pass_iv = defaultdict(list)
        by_buck_iv = defaultdict(list)
        for r in rs:
            by_pass_iv[r["pass"]].append((r["cstart"], r["cend"]))
            by_buck_iv[bucket_for(r["pass"])].append((r["cstart"], r["cend"]))
        for p, iv in by_pass_iv.items():
            frame_pass[p].append(merge_union(iv) / 1e6)
        for b, iv in by_buck_iv.items():
            frame_bucket[b].append(merge_union(iv) / 1e6)

    # Passes that don't appear every frame: pad with 0 for mean-over-frames
    nframes = len(by_frame)
    pass_stats = {}
    for p, xs in frame_pass.items():
        padded = xs + [0.0] * (nframes - len(xs))
        st = stats(padded)
        st["present_frames"] = len(xs)
        st["mean_when_present"] = sum(xs) / len(xs) if xs else None
        pass_stats[p] = st

    bucket_stats = {}
    for b, xs in frame_bucket.items():
        padded = xs + [0.0] * (nframes - len(xs))
        st = stats(padded)
        st["present_frames"] = len(xs)
        bucket_stats[b] = st

    # GPU performance state time-weighted in window
    pstate_ms = defaultdict(float)
    for start, dur, state in pstate:
        s = max(start, w0)
        e = min(start + dur, w1)
        if e > s:
            pstate_ms[state or "?"] += (e - s) / 1e6

    return {
        "n_gpu_intervals": len(gpu),
        "n_frames": nframes,
        "unlabeled_frames": unlabeled_frames,
        "frame_grouping": "vsync-bin" if use_bins else "frame-number",
        "collapsed_frame_numbers": collapsed,
        "gpu_union_ms": stats(frame_union),
        "gpu_sum_ms": stats(frame_sum),
        "passes": pass_stats,
        "buckets": bucket_stats,
        "pstate_ms": dict(pstate_ms),
        "frame_union": frame_union,
        "frame_sum": frame_sum,
    }


def write_pass_csv(path, pass_stats):
    rows = sorted(pass_stats.items(), key=lambda kv: -(kv[1]["mean"] or 0))
    with open(path, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["pass", "bucket", "n_frames_present", "mean_ms", "p50_ms", "p95_ms", "max_ms", "mean_when_present_ms"])
        for name, st in rows:
            w.writerow([
                name,
                bucket_for(name),
                st.get("present_frames", st["n"]),
                f"{st['mean']:.4f}" if st["mean"] is not None else "",
                f"{st['p50']:.4f}" if st["p50"] is not None else "",
                f"{st['p95']:.4f}" if st["p95"] is not None else "",
                f"{st['max']:.4f}" if st["max"] is not None else "",
                f"{st['mean_when_present']:.4f}" if st.get("mean_when_present") is not None else "",
            ])


def md_table(headers, rows):
    lines = ["| " + " | ".join(headers) + " |",
             "| " + " | ".join("---" for _ in headers) + " |"]
    for r in rows:
        lines.append("| " + " | ".join(str(x) for x in r) + " |")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--stem", help="prefix of the exported <stem>-<table>.xml files")
    ap.add_argument("--out-stem", help="output prefix (default: --stem)")
    ap.add_argument("--encoders")
    ap.add_argument("--gpu")
    ap.add_argument("--pstate")
    ap.add_argument("--toc")
    ap.add_argument("--events")
    ap.add_argument("--scenario", default="")
    ap.add_argument("--pid", type=int)
    ap.add_argument("--window-last-s", type=float, default=25.0,
                    help="If events marks missing, use last N seconds of GPU activity")
    args = ap.parse_args()

    stem = args.stem
    encoders_path = args.encoders or (f"{stem}-metal-application-encoders-list.xml" if stem else None)
    gpu_path = args.gpu or (f"{stem}-metal-gpu-intervals.xml" if stem else None)
    pstate_path = args.pstate or (f"{stem}-gpu-performance-state-intervals.xml" if stem else None)
    toc_path = args.toc or (f"{stem}-toc.xml" if stem else None)
    events_path = args.events or (f"{stem}-events.jsonl" if stem else None)
    if not encoders_path or not gpu_path:
        ap.error("need --stem or --encoders/--gpu")
    out_stem = args.out_stem or stem
    if not out_stem:
        ap.error("need --stem or --out-stem")

    pid = args.pid or toc_pid(toc_path)
    print(f"pid={pid}", file=sys.stderr)
    print(f"parsing encoders {encoders_path}", file=sys.stderr)
    encoders, labels_seen = parse_encoders(encoders_path, pid_filter=pid)
    print(f"  {len(encoders)} encoders, {len(labels_seen)} labels", file=sys.stderr)
    print(f"parsing gpu intervals {gpu_path}", file=sys.stderr)
    gpu_rows = parse_gpu_intervals(gpu_path, pid_filter=pid)
    print(f"  {len(gpu_rows)} game GPU intervals", file=sys.stderr)
    pstate = []
    if pstate_path and os.path.exists(pstate_path):
        print(f"parsing pstate {pstate_path}", file=sys.stderr)
        pstate = parse_pstate(pstate_path)
        print(f"  {len(pstate)} pstate intervals", file=sys.stderr)

    ev = parse_events_window(events_path, args.scenario) if events_path else None
    toc_dur = toc_duration(toc_path)

    # metal-flight.sh attaches when the measure scenario starts, so the first
    # recorded GPU work is ~the start mark. The window is that plus the
    # scenario's game-time length, clipped to what was recorded.
    window_note = ""
    if gpu_rows:
        gmin = min(r["start_ns"] for r in gpu_rows)
        gmax = max(r["end_ns"] for r in gpu_rows)
    else:
        gmin, gmax = 0, 0

    if ev and ev["start"] is not None and ev["end"] is not None and gpu_rows:
        span_ns = int((ev["end"] - ev["start"]) * 1e9)
        w0 = gmin
        w1 = min(gmax, gmin + span_ns)
        window_note = (
            f"events scenario {args.scenario} [{ev['start']:.3f},{ev['end']:.3f}]s "
            f"({span_ns/1e9:.1f}s) -> trace [{w0/1e9:.3f},{w1/1e9:.3f}]s"
            f" of {toc_dur or 0:.1f}s recorded"
        )
        # ~1 s of attach latency is normal; only flag a real shortfall.
        if w1 < gmin + span_ns - 2_000_000_000:
            window_note += " (recording shorter than the scenario; raise MAYBRAID_GPU_TRACE_LIMIT)"
    else:
        w1 = gmax
        w0 = max(gmin, gmax - int(args.window_last_s * 1e9))
        window_note = f"no/partial events marks; using last {args.window_last_s:.1f}s of GPU [{w0/1e9:.3f},{w1/1e9:.3f}]s"

    print(window_note, file=sys.stderr)
    result = analyze(encoders, gpu_rows, pstate, (w0, w1), pid)

    out = {
        "pid": pid,
        "scenario": args.scenario,
        "window_note": window_note,
        "events": ev,
        "toc_duration_s": toc_dur,
        "encoder_labels": dict(sorted(labels_seen.items(), key=lambda kv: -kv[1])),
        "n_gpu_intervals": result["n_gpu_intervals"],
        "n_frames": result["n_frames"],
        "gpu_union_ms": result["gpu_union_ms"],
        "gpu_sum_ms": result["gpu_sum_ms"],
        "passes": result["passes"],
        "buckets": result["buckets"],
        "pstate_ms": result["pstate_ms"],
    }

    json_path = f"{out_stem}-stats.json"
    csv_path = f"{out_stem}-passes.csv"
    md_path = f"{out_stem}-summary.md"

    with open(json_path, "w") as f:
        json.dump(out, f, indent=2, default=str)
    write_pass_csv(csv_path, result["passes"])

    gu = result["gpu_union_ms"]
    gs = result["gpu_sum_ms"]
    lines = []
    lines.append(f"# {args.scenario or os.path.basename(stem or 'run')}")
    lines.append("")
    lines.append(window_note)
    if result.get("collapsed_frame_numbers"):
        lines.append(
            "Frame grouping fell back to 16.67 ms bins: Metal `frame-number` was unique per GPU interval "
            "([#1198](https://github.com/ramate-io/maybraid/issues/1198))."
        )
    if ev and ev.get("actors"):
        a = ev["actors"]
        lines.append(
            f"Last actors census: hosts={a.get('hosts')} bodies={a.get('bodies')} brains={a.get('brains')}"
        )
    lines.append("")
    lines.append("## GPU time per frame (ms)")
    lines.append("")
    lines.append(
        md_table(
            ["metric", "n", "mean", "p50", "p95", "max"],
            [
                ["GPU busy (union of channels)", gu["n"], fmt_ms(gu["mean"]), fmt_ms(gu["p50"]), fmt_ms(gu["p95"]), fmt_ms(gu["max"])],
                ["GPU channel-sum (Vertex+Fragment+…)", gs["n"], fmt_ms(gs["mean"]), fmt_ms(gs["p50"]), fmt_ms(gs["p95"]), fmt_ms(gs["max"])],
            ],
        )
    )
    lines.append("")
    lines.append("Union is wall-clock GPU occupancy (parallel channels counted once) and is the number to compare against the 16.67 ms frame budget. Channel-sum can exceed occupancy when Vertex and Fragment overlap.")
    lines.append("")
    lines.append("## Pass buckets (mean GPU ms/frame, union within bucket)")
    lines.append("")
    brows = []
    for b, st in sorted(result["buckets"].items(), key=lambda kv: -(kv[1]["mean"] or 0)):
        brows.append([b, st.get("present_frames"), fmt_ms(st["mean"]), fmt_ms(st["p50"]), fmt_ms(st["p95"]), fmt_ms(st["max"])])
    lines.append(md_table(["bucket", "frames present", "mean", "p50", "p95", "max"], brows))
    lines.append("")
    lines.append("## Top encoder/pass labels (mean GPU ms/frame)")
    lines.append("")
    prows = []
    top = sorted(result["passes"].items(), key=lambda kv: -(kv[1]["mean"] or 0))[:25]
    for name, st in top:
        prows.append([
            f"`{name}`",
            bucket_for(name),
            st.get("present_frames"),
            fmt_ms(st["mean"]),
            fmt_ms(st["p95"]),
            fmt_ms(st["max"]),
        ])
    lines.append(md_table(["pass", "bucket", "frames", "mean", "p95", "max"], prows))
    lines.append("")
    lines.append("## GPU performance state (time-weighted in window)")
    lines.append("")
    total_ps = sum(result["pstate_ms"].values()) or 1.0
    psrows = []
    for st, ms in sorted(result["pstate_ms"].items(), key=lambda kv: -kv[1]):
        psrows.append([st, f"{ms:.1f}", f"{100*ms/total_ps:.1f}%"])
    if psrows:
        lines.append(md_table(["state", "ms", "share"], psrows))
    else:
        lines.append("_no pstate table_")
    lines.append("")
    lines.append("Apple GPUs downclock when lightly loaded; GPU ms at Minimum/Low overstates cost relative to a fully clocked chip.")
    with open(md_path, "w") as f:
        f.write("\n".join(lines) + "\n")
    print(f"wrote {json_path}", file=sys.stderr)
    print(f"wrote {csv_path}", file=sys.stderr)
    print(f"wrote {md_path}", file=sys.stderr)
    print(open(md_path).read())


if __name__ == "__main__":
    main()
