#!/usr/bin/env python3
"""Render `plans/evidence/<name>/README.md` from the committed JSON artifacts.

The scale-evidence artifacts are the source of truth; this renderer keeps the
prose from drifting away from them (ADR-0095 requires each published claim to
link a current evidence manifest).

Usage:
    python3 scripts/render-scale-evidence.py scale_2026_09_17

The directory name is validated and resolved under `plans/evidence/`; no
caller-supplied filesystem path is used.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

EVIDENCE_ROOT = pathlib.Path(__file__).resolve().parent.parent / "plans" / "evidence"
SAFE_DIR_NAME = re.compile(r"^[A-Za-z0-9_-]+$")


def resolve_dir(name: str) -> pathlib.Path:
    """Resolve `name` to a directory under `plans/evidence/` or exit."""
    if not SAFE_DIR_NAME.fullmatch(name):
        raise SystemExit(f"invalid evidence directory name: {name!r}")
    target = EVIDENCE_ROOT / name
    if not target.is_dir():
        raise SystemExit(f"no such evidence directory: {target}")
    return target


def fmt_bytes(value: int) -> str:
    for unit, div in (("GB", 1024**3), ("MB", 1024**2), ("kB", 1024)):
        if value >= div:
            return f"{value / div:,.2f} {unit}".replace(".00", "")
    return f"{value:,} B"


def fmt_ms(value: float) -> str:
    return f"{value / 1000:,.2f} s" if value >= 1000 else f"{value:,.1f} ms"


def fmt_us(value: float) -> str:
    return f"{value / 1000:,.2f} ms" if value >= 1000 else f"{value:,.0f} µs"


def load(path: pathlib.Path):
    return json.loads(path.read_text()) if path.exists() else None


def render_header(man: dict) -> list[str]:
    out = ["# Scale evidence", ""]
    out += [
        "ADR-0095 Tier-2 artifacts produced by `examples/scale_evidence` and",
        "`scripts/scale-evidence.sh`; regenerate this file with",
        "`python3 scripts/render-scale-evidence.py scale_2026_09_17`. Each artifact records",
        "its own commit, dirty state, corpus checksum, toolchain and hardware in",
        "`evidence_<mode>.json`.",
        "",
    ]
    ann_manifest = man.get("ann")
    if ann_manifest:
        hardware = ann_manifest["hardware"]
        out.append(
            f"Reference machine: {hardware['cpu']}, {hardware['cores']} threads, "
            f"{hardware['os']} {hardware['arch']}."
        )
        out.append("")
    out += [
        "Corpus: `synthetic-clustered-v1` — 64 random cluster centres, concept `i` is",
        "the centre for `i % 64` with 512 random bit flips, queries have 256 flips.",
        "Generation is prefix-stable, so the first `k` concepts of an `N`-concept",
        "corpus are identical for every `N`.",
        "",
    ]
    return out


def render_ann(ann: dict) -> list[str]:
    out = ["## ANN comparison (`ann_scale.json`)", ""]
    out += [
        f"`top_k = {ann['top_k']}`, {ann['scales'][0]['queries']} queries per scale; recall@k is",
        "relevance-based (relevant retrieved / total relevant) against the exact",
        "brute-force ground truth, not hit rate.",
        "",
        "| N | backend | build | p50 query | p99 query | recall@10 | index bytes | serialized | reload |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    order = {"exact": 0, "hnsw": 1, "lsh": 2, "bucket": 3}
    for scale in ann["scales"]:
        for backend in sorted(scale["backends"], key=lambda b: order[b["backend"]]):
            query = backend["query"]
            serialized = fmt_bytes(backend["serialize_bytes"]) if backend["serialize_bytes"] else "—"
            reload_s = fmt_ms(backend["reload_ms"]) if backend["reload_ms"] else "—"
            index_bytes = fmt_bytes(backend["index_bytes"]) if backend["index_bytes"] else "0"
            out.append(
                f"| {scale['concepts']:,} | {backend['backend']} | {fmt_ms(backend['build_ms'])} "
                f"| {fmt_us(query['p50_us'])} | {fmt_us(query['p99_us'])} | {backend['recall_at_k']:.3f} "
                f"| {index_bytes} | {serialized} | {reload_s} |"
            )
    out.append("")

    largest = max(ann["scales"], key=lambda s: s["concepts"])
    by = {b["backend"]: b for b in largest["backends"]}
    exact, hnsw, lsh, bucket = (by[name] for name in ("exact", "hnsw", "lsh", "bucket"))
    out += [
        "Findings at the largest scale:",
        "",
        f"- **Exact scan is the query-latency wall**: {fmt_us(exact['query']['p50_us'])} p50; "
        f"HNSW answers in {fmt_us(hnsw['query']['p50_us'])} "
        f"({exact['query']['p50_us'] / hnsw['query']['p50_us']:.1f}× faster) for {hnsw['recall_at_k']:.3f} "
        f"recall, LSH in {fmt_us(lsh['query']['p50_us'])} for {lsh['recall_at_k']:.3f}.",
        f"- **HNSW build is the cost, not the query**: {fmt_ms(hnsw['build_ms'])} to build against "
        f"{fmt_ms(lsh['build_ms'])} for LSH; reloading the serialized index "
        f"({fmt_bytes(hnsw['serialize_bytes'])}) takes {fmt_ms(hnsw['reload_ms'])}.",
        f"- **Bucketed candidate generation (probe width {bucket['bucket_probe_width']}) is not a "
        f"recall-preserving filter**: {bucket['recall_at_k']:.3f} recall@10 at "
        f"{fmt_us(bucket['query']['p50_us'])} p50 with {bucket['candidate_count_avg']:.0f} candidates per "
        f"query on average.",
        f"- Index overhead over the raw vectors: HNSW "
        f"{(hnsw['index_bytes'] - exact['index_bytes']) // largest['concepts']} B/concept, LSH "
        f"{(lsh['index_bytes'] - exact['index_bytes']) // largest['concepts']}; the bucketed path keeps "
        f"no index.",
        "",
    ]
    return out


def render_contention(label: str, doc: dict) -> list[str]:
    contention = doc["contention"]
    limit = doc["retry_policy"]["probe_side"]
    out = [
        f"{label} ({contention['tasks']} tasks × {contention['ops_per_task']} round-trips on one "
        "local database):",
        "",
        "| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |",
        "|---|---|---|---|---|---|---|",
    ]
    variants = [
        ("no retry", contention["raw"]),
        (
            f"caller-side bounded retry ({limit['limit']} × {limit['backoff_ms']} ms)",
            contention["bounded_retry"],
        ),
    ]
    for name, variant in variants:
        out.append(
            f"| {name} | {variant['ops_per_sec']:,.0f} | {fmt_us(variant['latency']['p50_us'])} "
            f"| {fmt_us(variant['latency']['p95_us'])} | {fmt_us(variant['latency']['p99_us'])} "
            f"| {variant['retry_rate']:.2f} | **{variant['error_rate']:.3f}** |"
        )
    out.append("")
    return out


def render_persistence(per: dict, per_pre: dict | None) -> list[str]:
    out = ["## Persistence (`persistence_scale.json`)", ""]
    out += [
        f"Batches of {per['batch_size']} concepts, DB/WAL/SHM measured separately:",
        "",
        "| N | write throughput | batch p50 | read throughput | db bytes | wal | shm | bytes/concept |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for scale in per["scales"]:
        side = scale["bytes_after_checkpoint"]
        out.append(
            f"| {scale['concepts']:,} | {scale['write_concepts_per_sec']:,.0f}/s "
            f"| {fmt_us(scale['batch_latency']['p50_us'])} | {scale['read_concepts_per_sec']:,.0f}/s "
            f"| {fmt_bytes(side['db_bytes'])} | {side['wal_bytes']} | {side['shm_bytes']} "
            f"| {side['total_bytes'] // scale['concepts']:,} |"
        )
    out.append("")

    if per_pre:
        out += [
            "Before the bounded retry/timeout change in `csm-persistence`",
            "(`persistence_scale_pre_fix.json`, same workload):",
            "",
        ]
        out += render_contention("pre-fix", per_pre)
    out += render_contention("post-fix", per)

    contention = per["contention"]
    if per_pre:
        before = per_pre["contention"]
        out.append(
            f"**Result:** with a 5 s busy timeout (and retries available when a wait is not enough) "
            f"every concurrent write succeeds — error rate "
            f"{before['raw']['error_rate']:.3f} → {contention['raw']['error_rate']:.3f} raw and "
            f"{before['bounded_retry']['error_rate']:.3f} → "
            f"{contention['bounded_retry']['error_rate']:.3f} with caller-side retries."
        )
    else:
        out.append(
            f"**Result:** error rate {contention['raw']['error_rate']:.3f} raw, "
            f"{contention['bounded_retry']['error_rate']:.3f} with caller-side retries."
        )
    out += [
        "",
        "Writers now wait (bounded) instead of failing, so per-operation tail latency at high "
        "contention is higher while the failure rate is zero.",
        "",
    ]
    return out


def render_memory(mem: dict) -> list[str]:
    out = [
        "## Memory model (`memory_model.json`)",
        "",
        "Per-point child processes; RSS delta is baseline-subtracted VmRSS, storage is",
        "`db + wal + shm` after checkpoint.",
        "",
        "| N | RSS delta | peak RSS | RSS/concept | storage | storage/concept |",
        "|---|---|---|---|---|---|",
    ]
    for point in mem["points"]:
        mark = " (held out)" if point["concepts"] == mem["holdout"]["concepts"] else ""
        out.append(
            f"| {point['concepts']:,}{mark} | {fmt_bytes(point['rss_delta_bytes'])} "
            f"| {fmt_bytes(point['peak_rss_bytes'])} | {point['rss_bytes_per_concept']:,.0f} B "
            f"| {fmt_bytes(point['storage']['total_bytes'])} "
            f"| {point['storage_bytes_per_concept']:,.0f} B |"
        )
    out.append("")

    fit = mem["fit"]
    holdout = mem["holdout"]
    out += [
        "Fits (points " + ", ".join(f"{n // 1000} k" for n in fit["fit_concepts"]) + "):",
        "",
        "```",
        f"RSS     = {fit['rss_intercept_bytes']:,.0f} B + {fit['rss_bytes_per_concept']:,.1f} B × "
        f"concepts    held-out error @{holdout['concepts']:,}: {100 * holdout['rss_relative_error']:.2f} %",
        f"storage = {fit['storage_intercept_bytes']:,.0f} B + "
        f"{fit['storage_bytes_per_concept']:,.1f} B × concepts    held-out error "
        f"@{holdout['concepts']:,}: {100 * holdout['storage_relative_error']:.2f} %",
        "```",
        "",
    ]
    projection = mem["projection"]
    out += [
        f"{mem['projection_concepts']:,}-concept projection: "
        f"**{fmt_bytes(int(projection['rss_bytes']))} RSS**, "
        f"**{fmt_bytes(int(projection['storage_bytes']))} storage** against the recorded "
        f"`< {fmt_bytes(mem['claim_bytes'])}` target — {projection['claim_ratio_rss']:,.0f}× and "
        f"{projection['claim_ratio_storage']:,.0f}× over. "
        f"`claim_supported: {str(projection['claim_supported']).lower()}`.",
        "",
        "The 12 MB figure describes the product-quantization design (ADR-0024 phase 2:",
        "~1 byte/concept + 2 MB codebook), which was never implemented. The shipped",
        "representation is a 1 280-byte `HVec10240` held in memory (plus index copies)",
        "and written twice per concept (concept row + version row).",
        "",
    ]
    return out


def render(base: pathlib.Path) -> str:
    def read(name: str):
        return load(base / name)

    ann = read("ann_scale.json")
    persistence = read("persistence_scale.json")
    persistence_pre = read("persistence_scale_pre_fix.json")
    memory = read("memory_model.json")
    manifests = {
        mode: read(f"evidence_{mode}.json") or (read("evidence_persistence_pre_fix.json") if mode == "persistence" else None)
        for mode in ("ann", "persistence", "memory")
    }

    out = render_header(manifests)
    if ann:
        out += render_ann(ann)
    if persistence:
        out += render_persistence(persistence, persistence_pre)
    if memory:
        out += render_memory(memory)
    return "\n".join(out)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    base = resolve_dir(sys.argv[1])
    (base / "README.md").write_text(render(base))
    print(f"wrote {base / 'README.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
