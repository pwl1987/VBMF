#!/usr/bin/env python3
"""Step 15 长稳 v2 谓词分析（参数化推广版）。

用法: python3 reanalyze-long.py <evidence_dir> <tick_mid> <tick_end> <expected_cycles>

派生自 reanalyze-v2.py（原文件字节不动·R64-6' 已登记工件）: 判定逻辑逐字
保持, 仅把谓词 4/5/6 钉死的周期数 60 参数化为 expected_cycles。十项阈值
常量（threads spread<=4 / fd+8 / RSS+50MB 无单调等）不随时长放宽。
响应词表: status.status=executed/replayed, detail/classification 为顶层字段。
对**未改动的原始证据**做纯后处理重析。
"""
import csv
import json
import re
import sys

ev, tick_mid, tick_end, n_exp = sys.argv[1], int(sys.argv[2] or 0), int(sys.argv[3] or 0), int(sys.argv[4] or 0)
rows = list(csv.DictReader(open(f"{ev}/samples.csv")))
verdicts = []


def pred(name, ok, note=""):
    verdicts.append((name, ok, note))
    print(f"{'PASS' if ok else 'FAIL'} {name} {note}")


ths = [int(r["threads"]) for r in rows]
pred("threads_bounded_oscillation", max(ths) - min(ths) <= 4,
     f"min={min(ths)} max={max(ths)} spread={max(ths)-min(ths)}")
fds = [int(r["fd"]) for r in rows]
head, tail = fds[:3], fds[-3:]
pred("fd_bounded", sum(tail) / 3 <= sum(head) / 3 + 8,
     f"first3_avg={sum(head)/3:.1f} last3_avg={sum(tail)/3:.1f}")
rss = [int(r["rss_kb"]) for r in rows]
n3 = len(rss) // 3
mono = all(rss[i + 1] > rss[i] for i in range(len(rss) - 1))
pred("rss_bounded", sum(rss[-n3:]) / n3 <= sum(rss[:n3]) / n3 + 50 * 1024 and not mono,
     f"first_third={sum(rss[:n3])/n3/1024:.1f}MB last_third={sum(rss[-n3:])/n3/1024:.1f}MB monotonic={mono}")
sws = [int(r["sw_epoch"]) for r in rows]
per_cmd = all(sws[i + 1] == sws[i] + 1 for i in range(len(sws) - 1))
pred("switch_epoch_per_command_plus1", per_cmd and sws[0] == 1 and sws[-1] == n_exp,
     f"first={sws[0]} last={sws[-1]} expected={n_exp} consecutive_plus1={per_cmd}")
ok_cmds = 0
for line in open(f"{ev}/switch-responses.jsonl"):
    line = line.strip()
    if not line:
        continue
    j = json.loads(line)
    if (j.get("status") or {}).get("status") == "executed" and "outcome=preserved" in (j.get("detail") or ""):
        ok_cmds += 1
pred("switches_all_executed_preserved", ok_cmds == n_exp, f"ok={ok_cmds}/{n_exp}")
rb_ok, rb_total = 0, 0
pat = re.compile(r"cycle=(\d+) target=(\S+) observed=(\S+)")
for line in open(f"{ev}/readbacks.txt"):
    m = pat.search(line)
    if m:
        rb_total += 1
        if m.group(3) == m.group(2):
            rb_ok += 1
pred("observed_tracks_per_command", rb_total == n_exp and rb_ok == n_exp, f"match={rb_ok}/{rb_total} expected={n_exp}")
fv = [int(r["frames_v"]) for r in rows]
fa = [int(r["frames_a"]) for r in rows]
pred("frames_advancing",
     all(fv[i + 1] > fv[i] for i in range(len(fv) - 1)) and all(fa[i + 1] > fa[i] for i in range(len(fa) - 1)),
     f"v:{fv[0]}->{fv[-1]} a:{fa[0]}->{fa[-1]}")
pred("drops_zero", all(int(r["dropped"]) == 0 for r in rows) and all(int(r["clock_lost"]) == 0 for r in rows))
pred("watchdog_ticks_advancing", tick_end > tick_mid > 0, f"mid={tick_mid} end={tick_end}")
try:
    ev_lines = [l for l in open(f"{ev}/events.txt") if "critical" in l]
    crit_ok = all("critical=0" in l or "critical=False" in l for l in ev_lines)
    pred("events_no_critical", crit_ok, f"samples={len(ev_lines)}")
except FileNotFoundError:
    pred("events_no_critical", False, "events.txt missing")
rp_lines = [l for l in open(f"{ev}/replays.jsonl") if l.strip()]
rp_replayed = sum(1 for l in rp_lines if '"replayed"' in l)
print(f"NOTE replays={len(rp_lines)} replayed_status={rp_replayed} (data, not gating)")
allp = all(ok for _, ok, _ in verdicts)
print(f"VERDICT {'PASS' if allp else 'FAIL'} ({sum(1 for _, ok, _ in verdicts if ok)}/{len(verdicts)} predicates v2)")
