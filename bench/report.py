"""Aggregate timings, metrics, and judge verdicts into out/report.md."""

import json
import statistics
from collections import defaultdict
from pathlib import Path

from convert import OUT


def load_timings():
    latest = {}
    path = OUT / "timings.jsonl"
    if path.exists():
        for line in path.read_text(encoding="utf-8").splitlines():
            r = json.loads(line)
            latest[(r["tool"], r["stem"])] = r
    return list(latest.values())


def speed_section(rows):
    lines = ["## Speed", "",
             "Warm timings; Python tools in-process, CLI tools include process spawn.", ""]
    by_fmt = defaultdict(lambda: defaultdict(list))
    for r in rows:
        if r.get("ok"):
            by_fmt[r["format"]][r["tool"]].append(r)
    for fmt in sorted(by_fmt):
        lines += [f"### {fmt}", "", "| tool | files | total MB | median ms | MB/s |", "| --- | --- | --- | --- | --- |"]
        for tool, rs in sorted(by_fmt[fmt].items(), key=lambda kv: statistics.median(x["min_ms"] for x in kv[1])):
            total_mb = sum(x["bytes"] for x in rs) / 1e6
            total_s = sum(x["min_ms"] for x in rs) / 1000
            med = statistics.median(x["min_ms"] for x in rs)
            mbs = total_mb / total_s if total_s else 0
            lines.append(f"| {tool} | {len(rs)} | {total_mb:.1f} | {med:.1f} | {mbs:.1f} |")
        lines.append("")
    fails = [r for r in rows if not r.get("ok")]
    if fails:
        lines += ["### Failures", ""]
        by_tool = defaultdict(int)
        for r in fails:
            by_tool[r["tool"]] += 1
        for tool, n in sorted(by_tool.items()):
            lines.append(f"- {tool}: {n} file(s) failed")
        lines.append("")
    return lines


def judge_section():
    path = OUT / "judge.jsonl"
    if not path.exists():
        return []
    verdicts = defaultdict(dict)
    for line in path.read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        verdicts[(r["stem"], r["format"], r["opponent"])][r["order"]] = r["winner"]

    tallies = defaultdict(lambda: [0, 0, 0])  # win, tie, loss for anydoc
    for (stem, fmt, opp), orders in verdicts.items():
        winners = set(orders.values())
        if len(orders) < 2 or len(winners) > 1:
            outcome = 1  # inconsistent across positions, or single verdict: tie
            if len(orders) < 2:
                w = next(iter(winners))
                outcome = 0 if w == "anydoc" else (1 if w == "tie" else 2)
        else:
            w = winners.pop()
            outcome = 0 if w == "anydoc" else (1 if w == "tie" else 2)
        tallies[(fmt, opp)][outcome] += 1

    lines = ["## LLM judge (anydoc vs opponent)", "",
             "Each doc judged twice with positions swapped; disagreement counts as a tie.", "",
             "| format | opponent | anydoc wins | ties | losses | win rate (excl. ties) |",
             "| --- | --- | --- | --- | --- | --- |"]
    for (fmt, opp), (w, t, l) in sorted(tallies.items()):
        rate = f"{w / (w + l):.0%}" if (w + l) else "-"
        lines.append(f"| {fmt} | {opp} | {w} | {t} | {l} | {rate} |")
    lines.append("")
    return lines


def metrics_section():
    path = OUT / "metrics.json"
    if not path.exists():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    lines = ["## Content recall vs anydoc (word-trigram containment)", "",
             "| tool | docs | anydoc content found in tool | tool content found in anydoc |",
             "| --- | --- | --- | --- |"]
    for tool, docs in sorted(data.items()):
        pairs = [(d["anydoc_recall_here"], d["here_recall_anydoc"])
                 for d in docs.values() if "anydoc_recall_here" in d]
        if not pairs:
            continue
        a = statistics.fmean(p[0] for p in pairs)
        b = statistics.fmean(p[1] for p in pairs)
        lines.append(f"| {tool} | {len(pairs)} | {a:.2f} | {b:.2f} |")
    lines.append("")
    return lines


def main():
    rows = load_timings()
    lines = ["# anydoc benchmark report", ""]
    lines += speed_section(rows)
    lines += judge_section()
    lines += metrics_section()
    report = "\n".join(lines)
    (OUT / "report.md").write_text(report, encoding="utf-8")
    print(report)


if __name__ == "__main__":
    main()
