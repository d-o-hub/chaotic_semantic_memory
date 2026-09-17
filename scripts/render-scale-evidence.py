#!/usr/bin/env python3
"""Render `plans/evidence/*/README.md` from the committed JSON artifacts.

The scale-evidence artifacts are the source of truth; this renderer keeps the
prose from drifting away from them (ADR-0095 requires each published claim to
link a current evidence manifest).

Usage:
    python3 scripts/render-scale-evidence.py plans/evidence/scale_2026_09_17
"""

from __future__ import annotations

import json
import pathlib
import sys


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


def render(base: pathlib.Path) -> str:
    ann = load(base / "ann_scale.json")
    per = load(base / "persistence_scale.json")
    per_pre = load(base / "persistence_scale_pre_fix.json")
    mem = load(base / "memory_model.json")
    man = {name: load(base / f"evidence_{name}.json") for name in ("ann", "persistence", "memory")}
    man["persistence"] = man["persistence"] or load(base / "evidence_persistence_pre_fix.json")

    out: list[str] = []
    add = out.append

    add("# Scale evidence")
    add("")
    add("ADR-0095 Tier-2 artifacts produced by `examples/scale_evidence` and")
    add("`scripts/scale-evidence.sh`; regenerate this file with")
    add("`python3 scripts/render-scale-evidence.py <this directory>`. Each artifact records")
    add("its own commit, dirty state, corpus checksum, toolchain and hardware in")
    add("`evidence_<mode>.json`.")
    add("")

    if ann:
        hw = man["ann"]["hardware"]
        add(f"Reference machine: {hw['cpu']}, {hw['cores']} threads, {hw['os']} {hw['arch']}.")
        add("")

    add("Corpus: `synthetic-clustered-v1` — 64 random cluster centres, concept `i` is")
    add("the centre for `i % 64` with 512 random bit flips, queries have 256 flips.")
    add("Generation is prefix-stable, so the first `k` concepts of an `N`-concept")
    add("corpus are identical for every `N`.")
    add("")

    if ann:
        add("## ANN comparison (`ann_scale.json`)")
        add("")
        add(f"`top_k = {ann['top_k']}`, {ann['scales'][0]['queries']} queries per scale; recall@k is")
        add("relevance-based (relevant retrieved / total relevant) against the exact")
        add("brute-force ground truth, not hit rate.")
        add("")
        add("| N | backend | build | p50 query | p99 query | recall@10 | index bytes | serialized | reload |")
        add("|---|---|---|---|---|---|---|---|---|")
        order = {"exact": 0, "hnsw": 1, "lsh": 2, "bucket": 3}
        for scale in ann["scales"]:
            for backend in sorted(scale["backends"], key=lambda b: order[b["backend"]]):
                query = backend["query"]
                serialized = fmt_bytes(backend["serialize_bytes"]) if backend["serialize_bytes"] else "—"
                reload_s = fmt_ms(backend["reload_ms"]) if backend["reload_ms"] else "—"
                add(
                    f"| {scale['concepts']:,} | {backend['backend']} | {fmt_ms(backend['build_ms'])} "
                    f"| {fmt_us(query['p50_us'])} | {fmt_us(query['p99_us'])} | {backend['recall_at_k']:.3f} "
                    f"| {fmt_bytes(backend['index_bytes']) if backend['index_bytes'] else '0'} "
                    f"| {serialized} | {reload_s} |"
                )
        add("")
        largest = max(ann["scales"], key=lambda s: s["concepts"])
        by = {b["backend"]: b for b in largest["backends"]}
        add("Findings at the largest scale:")
        add("")
        add(
            f"- **Exact scan is the query-latency wall**: {fmt_us(by['exact']['query']['p50_us'])} p50; "
            f"HNSW answers in {fmt_us(by['hnsw']['query']['p50_us'])} "
            f"({by['exact']['query']['p50_us'] / by['hnsw']['query']['p50_us']:.1f}× faster) for "
            f"{by['hnsw']['recall_at_k']:.3f} recall, LSH in {fmt_us(by['lsh']['query']['p50_us'])} "
            f"for {by['lsh']['recall_at_k']:.3f}."
        )
        add(
            f"- **HNSW build is the cost, not the query**: {fmt_ms(by['hnsw']['build_ms'])} to build against "
            f"{fmt_ms(by['lsh']['build_ms'])} for LSH; reloading the serialized index "
            f"({fmt_bytes(by['hnsw']['serialize_bytes'])}) takes {fmt_ms(by['hnsw']['reload_ms'])}."
        )
        add(
            f"- **Bucketed candidate generation (probe width {by['bucket']['bucket_probe_width']}) is not a "
            f"recall-preserving filter**: {by['bucket']['recall_at_k']:.3f} recall@10 at "
            f"{fmt_us(by['bucket']['query']['p50_us'])} p50 with {by['bucket']['candidate_count_avg']:.0f} "
            f"candidates per query on average."
        )
        add(
            f"- Index overhead over the raw vectors: HNSW "
            f"{(by['hnsw']['index_bytes'] - by['exact']['index_bytes']) // largest['concepts']} B/concept, LSH "
            f"{(by['lsh']['index_bytes'] - by['exact']['index_bytes']) // largest['concepts']}; the bucketed path "
            f"keeps no index."
        )
        add("")

    if per:
        add("## Persistence (`persistence_scale.json`)")
        add("")
        add(f"Batches of {per['batch_size']} concepts, DB/WAL/SHM measured separately:")
        add("")
        add("| N | write throughput | batch p50 | read throughput | db bytes | wal | shm | bytes/concept |")
        add("|---|---|---|---|---|---|---|---|")
        for scale in per["scales"]:
            b = scale["bytes_after_checkpoint"]
            add(
                f"| {scale['concepts']:,} | {scale['write_concepts_per_sec']:,.0f}/s "
                f"| {fmt_us(scale['batch_latency']['p50_us'])} | {scale['read_concepts_per_sec']:,.0f}/s "
                f"| {fmt_bytes(b['db_bytes'])} | {b['wal_bytes']} | {b['shm_bytes']} "
                f"| {b['total_bytes'] // scale['concepts']:,} |"
            )
        add("")

        def contention_table(label: str, doc) -> None:
            c = doc["contention"]
            limit = doc["retry_policy"]["probe_side"]
            add(f"{label} ({c['tasks']} tasks × {c['ops_per_task']} round-trips on one local database):")
            add("")
            add("| variant | ops/s | p50 | p95 | p99 | retries/op | error rate |")
            add("|---|---|---|---|---|---|---|")
            rows = [
                ("no retry", c["raw"]),
                (
                    f"caller-side bounded retry ({limit['limit']} × {limit['backoff_ms']} ms)",
                    c["bounded_retry"],
                ),
            ]
            for name, v in rows:
                add(
                    f"| {name} | {v['ops_per_sec']:,.0f} | {fmt_us(v['latency']['p50_us'])} "
                    f"| {fmt_us(v['latency']['p95_us'])} | {fmt_us(v['latency']['p99_us'])} "
                    f"| {v['retry_rate']:.2f} | **{v['error_rate']:.3f}** |"
                )
            add("")

        if per_pre:
            add("Before the bounded retry/timeout change in `csm-persistence`")
            add("(`persistence_scale_pre_fix.json`, same workload):")
            add("")
            contention_table("pre-fix", per_pre)
        contention_table("post-fix", per)
        c = per["contention"]
        add(
            f"**Result:** with a 5 s busy timeout (and retries available when a wait is not enough) every "
            f"concurrent write succeeds — error rate {per_pre['contention']['raw']['error_rate']:.3f} → "
            f"{c['raw']['error_rate']:.3f} raw and "
            f"{per_pre['contention']['bounded_retry']['error_rate']:.3f} → "
            f"{c['bounded_retry']['error_rate']:.3f} with caller-side retries."
            if per_pre
            else f"**Result:** error rate {c['raw']['error_rate']:.3f} raw, "
            f"{c['bounded_retry']['error_rate']:.3f} with caller-side retries."
        )
        add("")
        add(
            "Writers now wait (bounded) instead of failing, so per-operation tail latency at high contention "
            "is higher while the failure rate is zero."
        )
        add("")

    if mem:
        add("## Memory model (`memory_model.json`)")
        add("")
        add("Per-point child processes; RSS delta is baseline-subtracted VmRSS, storage is")
        add("`db + wal + shm` after checkpoint.")
        add("")
        add("| N | RSS delta | peak RSS | RSS/concept | storage | storage/concept |")
        add("|---|---|---|---|---|---|")
        for point in mem["points"]:
            mark = " (held out)" if point["concepts"] == mem["holdout"]["concepts"] else ""
            add(
                f"| {point['concepts']:,}{mark} | {fmt_bytes(point['rss_delta_bytes'])} "
                f"| {fmt_bytes(point['peak_rss_bytes'])} | {point['rss_bytes_per_concept']:,.0f} B "
                f"| {fmt_bytes(point['storage']['total_bytes'])} "
                f"| {point['storage_bytes_per_concept']:,.0f} B |"
            )
        add("")
        fit = mem["fit"]
        add("Fits (points " + ", ".join(f"{n // 1000} k" for n in fit["fit_concepts"]) + "):")
        add("")
        add("```")
        add(
            f"RSS     = {fit['rss_intercept_bytes']:,.0f} B + {fit['rss_bytes_per_concept']:,.1f} B × concepts"
            f"    held-out error @{mem['holdout']['concepts']:,}: {100 * mem['holdout']['rss_relative_error']:.2f} %"
        )
        add(
            f"storage = {fit['storage_intercept_bytes']:,.0f} B + {fit['storage_bytes_per_concept']:,.1f} B × concepts"
            f"    held-out error @{mem['holdout']['concepts']:,}: {100 * mem['holdout']['storage_relative_error']:.2f} %"
        )
        add("```")
        add("")
        proj = mem["projection"]
        add(
            f"{mem['projection_concepts']:,}-concept projection: **{fmt_bytes(int(proj['rss_bytes']))} RSS**, "
            f"**{fmt_bytes(int(proj['storage_bytes']))} storage** against the recorded "
            f"`< {fmt_bytes(mem['claim_bytes'])}` target — {proj['claim_ratio_rss']:,.0f}× and "
            f"{proj['claim_ratio_storage']:,.0f}× over. `claim_supported: {str(proj['claim_supported']).lower()}`."
        )
        add("")
        add("The 12 MB figure describes the product-quantization design (ADR-0024 phase 2:")
        add("~1 byte/concept + 2 MB codebook), which was never implemented. The shipped")
        add("representation is a 1 280-byte `HVec10240` held in memory (plus index copies)")
        add("and written twice per concept (concept row + version row).")
        add("")

    return "\n".join(out)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    base = pathlib.Path(sys.argv[1])
    (base / "README.md").write_text(render(base))
    print(f"wrote {base / 'README.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
