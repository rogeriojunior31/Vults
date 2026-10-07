#!/usr/bin/env python3
"""CPU and memory of a running Vults, its webview processes included.

Usage: scripts/perf.py [seconds]   (default 30)
Targets (plan): idle island 0 % CPU, compact < 3 %, RAM < 100 MB.
"""
import os, sys, time

TICK = os.sysconf("SC_CLK_TCK")


def procs():
    by_pid = {}
    for p in os.listdir("/proc"):
        if not p.isdigit():
            continue
        try:
            stat = open(f"/proc/{p}/stat").read()
            exe = os.readlink(f"/proc/{p}/exe")
        except OSError:
            continue
        rest = stat.rsplit(")", 1)[1].split()
        by_pid[int(p)] = (int(rest[1]), os.path.basename(exe))
    apps = {pid for pid, (_, exe) in by_pid.items() if exe == "vults"}
    # The app and every process below it (WebKit's web and network processes).
    family = set(apps)
    grew = True
    while grew:
        grew = False
        for pid, (ppid, _) in by_pid.items():
            if ppid in family and pid not in family:
                family.add(pid)
                grew = True
    return {pid: by_pid[pid][1] for pid in family}


def cpu_ticks(pid):
    try:
        rest = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
        return int(rest[11]) + int(rest[12])
    except OSError:
        return 0


def rss_mb(pid):
    try:
        for line in open(f"/proc/{pid}/status"):
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) / 1024
    except OSError:
        pass
    return 0.0


def pss_mb(pid):
    # PSS shares libraries fairly between processes: the honest total.
    try:
        for line in open(f"/proc/{pid}/smaps_rollup"):
            if line.startswith("Pss:"):
                return int(line.split()[1]) / 1024
    except OSError:
        pass
    return rss_mb(pid)


secs = float(sys.argv[1]) if len(sys.argv) > 1 else 30
family = procs()
if not family:
    sys.exit("Vults is not running")
before = {p: cpu_ticks(p) for p in family}
time.sleep(secs)
total_cpu = 0.0
total_pss = 0.0
for pid, name in sorted(family.items()):
    cpu = (cpu_ticks(pid) - before[pid]) / TICK / secs * 100
    pss = pss_mb(pid)
    total_cpu += cpu
    total_pss += pss
    print(f"{pid:>8} {name:<22} cpu {cpu:5.2f} %   pss {pss:6.1f} MB")
print(f"{'total':>8} {'':<22} cpu {total_cpu:5.2f} %   pss {total_pss:6.1f} MB   ({secs:.0f} s)")
