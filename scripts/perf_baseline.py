#!/usr/bin/env python3
"""Reproducible performance baseline: one command per workload.

    python3 scripts/perf_baseline.py WORKLOAD [--runs N] [--seed U64] [--out DIR] [--no-build]
    python3 scripts/perf_baseline.py --list

Each run launches the RELEASE game binary in measurement mode (`play --perf-script/--perf-report`)
on a private virtual X display (`xvfb-run`), with audio muted and fresh, throwaway save and options
directories, so it never touches a real display, the owner's saves or their options. The input
script drives the workload, the game writes a `key: value` report plus per-frame CSVs, and this
script checks that the workload did what it claims before printing the numbers.

What the reported numbers include (all over the measurement window only, except where noted):

- main_ms: wall time of one main-schedule frame, from just before `First` to just after `Last`
  (all gameplay, UI and transform systems). It excludes render-world extraction and the pacing
  sleep between frames.
- render_ms: wall time of one render-world `Render` schedule run on the pipelined render thread,
  from before `ExtractCommands` to after `PostCleanup`; it includes command encoding, queue submit
  and present. Rasterization by a software adapter (llvmpipe) may continue on driver threads and is
  then only visible in cpu_*.
- interval_ms: wall time between consecutive frame starts. It includes the pacing sleep, so on a
  software adapter it sits at the 30 Hz cap (33.3 ms) unless a frame overruns: its p99 and worst
  are the hitches a player sees.
- system_us NAME: per-call time of the `--timings` hotspot systems that ran in the window.
- cpu_s_per_s / cpu_ms_per_frame: process user+sys CPU time (every thread, including the render
  thread and the software rasterizer) over the window, per wall second and per frame. Linux only
  (/proc/self/stat, 10 ms tick resolution).
- peak_rss_mib: peak resident memory (VmHWM) over the whole process lifetime, read at exit;
  end_rss_mib is VmRSS at exit. Linux only.
- first_frame_ms (startup): launcher start (after CLI parsing) to the start of the first
  main-schedule frame: app construction, renderer and window creation, and the Startup schedules.
  The startup window then covers frames 2-301: pipeline compilation and the title's asset loads.

The leak-check workloads are different: a `snapshot LABEL` script step records, after each settled
map arrival, the ECS entity count, `Assets<T>::len()` for the relevant asset types and the
current resident memory (VmRSS) into `report.snapshots.csv`; the run ends with its script. The
summary prints that table per run and the least-squares RSS slope. They verify that every scripted
arrival was recorded. They measure retention, not speed, so their timing columns mean little.

Percentiles are nearest-rank, so each is a frame time that actually occurred. Measurements are only
comparable between runs on the same machine, build profile, pacing cap and seed; the header prints
all of them. Raw data stays under temp/ (gitignored).
"""

from __future__ import annotations

import argparse
import datetime
import os
import shutil
import statistics
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BINARY = REPO / "target" / "release" / "rpg-s1"
SAVE_FIXTURE = REPO / "tests" / "fixtures" / "native-save-v1.yaml"
XVFB_SCREEN = "-screen 0 1400x900x24"
RUN_TIMEOUT_SECONDS = 300

# The map with the most NPCs in the shipped scenario data: town_02_millhaven, town_04_frostholm and
# town_01_ardel tie at six; Frostholm is the largest of the three (61x35 tiles).
DENSE_NPC_MAP = "town_04_frostholm"

# leak-check: how many map arrivals the script makes and how many frames each destination gets
# to settle before its snapshot. Each loop walks one short hop through a portal, lands the player
# a short walk from the return portal, and walks back; the holds are tuned to the shipped maps
# and are verified by the arrivals the run must record.
LEAK_CHECK_CROSSINGS = 20
LEAK_CHECK_SETTLE_FRAMES = 120
LEAK_CHECK_LOOPS = {
    # name: (start map, start position, legs); crossing N takes leg (N-1) mod len(legs), each leg
    # being the walking steps that reach a portal and the map they arrive on.
    "leak-check": (
        "zone_01_starting_forest",
        "29,1",
        [(["hold 20 move_up"], "town_01_ardel"), (["hold 15 move_down"], "zone_01_starting_forest")],
    ),
    "leak-check-house": (
        "town_01_ardel",
        "3,5",
        [(["hold 15 move_up"], "town_01_ardel_house_01"), (["hold 12 move_down"], "town_01_ardel")],
    ),
    # Three distinct maps in six crossings: Ardel -> house -> Ardel -> shop -> Ardel -> house ->
    # Ardel. A portal lands the player on the tile just inside it, and the player walks 10 px
    # per frame at the 30 Hz cap.
    "leak-check-chain": (
        "town_01_ardel",
        "3,5",
        [
            (["hold 15 move_up"], "town_01_ardel_house_01"),
            (["hold 12 move_down"], "town_01_ardel"),
            (["hold 38 move_right", "hold 8 move_up"], "town_01_ardel_shop_01"),
            (["hold 12 move_down"], "town_01_ardel"),
            (
                ["hold 3 move_down", "hold 38 move_left", "hold 15 move_up"],
                "town_01_ardel_house_01",
            ),
            (["hold 12 move_down"], "town_01_ardel"),
        ],
    ),
}


def leak_check_script(start: str, legs: list, crossings: int) -> str:
    """Crosses a map boundary `crossings` times and snapshots after each settled arrival."""
    lines = [
        "wait_state World",
        f"wait_map {start}",
        f"wait {LEAK_CHECK_SETTLE_FRAMES}",
        # The window is only required by the script format; a snapshot run ends with its script.
        "measure 200000",
        f"snapshot start {start}",
    ]
    for crossing in range(1, crossings + 1):
        steps, destination = legs[(crossing - 1) % len(legs)]
        lines += [
            *steps,
            f"wait_map {destination}",
            "wait_state World",
            f"wait {LEAK_CHECK_SETTLE_FRAMES}",
            f"snapshot arrive {crossing:02d} {destination}",
        ]
    return "\n".join(lines) + "\n"


def leak_check_workload(name: str, crossings: int) -> dict:
    start, position, legs = LEAK_CHECK_LOOPS[name]
    maps = " -> ".join(dict.fromkeys([start, *(destination for _, destination in legs)]))
    return {
        "about": f"{crossings} portal crossings through {maps}; entity count, loaded assets per "
        "type and current RSS are recorded after each settled arrival (report.snapshots.csv); a "
        "debug session, so no autosave",
        "args": ["--start-map", start, "--start-position", position],
        "script": leak_check_script(start, legs, crossings),
        "expect_autosave": None,
        "expect_snapshots": crossings + 1,
        "timeout_s": 120 + 12 * crossings,
    }


WORKLOADS: dict[str, dict] = {
    "startup": {
        "about": "process launch to the interactive title screen; window opens on frame 1",
        "args": [],
        "script": """\
# The window covers window creation, pipeline compilation and the title load.
measure 300
wait_state Title
wait_text New Game
""",
        "metrics_extra": ["first_frame_ms"],
    },
    "idle": {
        "about": "standing still in Ardel (town_01_ardel 10,10), nothing pressed",
        "args": ["--start-map", "town_01_ardel", "--start-position", "10,10"],
        "script": """\
wait_state World
wait_map town_01_ardel
wait 60
measure 450
""",
    },
    "dense-npc": {
        "about": f"standing still in {DENSE_NPC_MAP}, the joint-largest NPC count (6) on the largest map",
        "args": ["--start-map", DENSE_NPC_MAP, "--start-position", "20,27"],
        "script": f"""\
wait_state World
wait_map {DENSE_NPC_MAP}
wait 60
measure 450
""",
    },
    "menu": {
        "about": "field menu: open, Status, Quests (list scroll), Save page, close; twice",
        "args": ["--start-map", "town_01_ardel", "--start-position", "10,10"],
        "script": """\
wait_state World
wait 60
measure 450
""" + """\
press back
wait_text PARTY COMMAND DECK
wait 10
press confirm
wait_text PARTY ROSTER AND GROWTH
wait 20
press back
wait_text PARTY COMMAND DECK
wait 5
press menu_down
wait 5
press confirm
wait_text QUEST BOARD
wait 5
press menu_down
wait 5
press menu_down
wait 5
press menu_up
wait 10
press back
wait_text PARTY COMMAND DECK
wait 5
press menu_down
wait 5
press menu_down
wait 5
press confirm
wait_text CHOOSE A RECORD
wait 20
press back
wait_text PARTY COMMAND DECK
wait 10
press back
wait 20
""" * 2,
    },
    "battle": {
        "about": "battle fixture `feedback` (Aric, Elise, Jep vs Goblin Warrior): two rounds of "
        "Attack; each result plays its float, flash and shake and waits for Confirm",
        "args": ["--battle-fixture", "feedback"],
        "script": """\
wait_state Battle
wait_text Attack
wait 30
measure 450
press confirm
wait_text Aric: choose an action.
""" + """\
wait 5
press confirm
wait_text Choose a target.
wait 5
press confirm
wait 30
press confirm
wait_text Elise: choose an action.
wait 5
press confirm
wait_text Choose a target.
wait 5
press confirm
wait 30
press confirm
wait_text Jep: choose an action.
wait 5
press confirm
wait_text Choose a target.
wait 5
press confirm
wait 30
press confirm
# The Goblin Warrior's turn resolves and waits for Confirm too.
wait 30
press confirm
wait_text Aric: choose an action.
""" * 2,
        "expect_entity_churn": True,
    },
    "transition": {
        "about": "walk from Starting Forest (29,1) north through the portal into Ardel; debug "
        "session, so no autosave",
        "args": ["--start-map", "zone_01_starting_forest", "--start-position", "29,1"],
        "script": """\
wait_state World
wait_map zone_01_starting_forest
wait 60
measure 300
hold 20 move_up
wait_map town_01_ardel
wait_state World
""",
        "expect_autosave": None,
    },
    "autosave": {
        "about": "load the fixture save from the title, then walk Starting Forest -> Ardel; the "
        "arrival writes autosave slot 0 inside the window",
        "args": [],
        "save_fixture": True,
        "script": """\
wait_state Title
wait_text Load Game
wait 10
press menu_down
wait 5
press confirm
wait_text Golden Aric
wait 10
press confirm
wait_state World
wait_map zone_01_starting_forest
# The first arrival writes an autosave here, before the window opens.
wait 90
measure 300
hold 20 move_up
wait_map town_01_ardel
wait_state World
""",
        "expect_autosave": "town_01_ardel",
    },
}

for _name in LEAK_CHECK_LOOPS:
    WORKLOADS[_name] = leak_check_workload(_name, LEAK_CHECK_CROSSINGS)
WORKLOADS["leak-check-long"] = leak_check_workload("leak-check", 3 * LEAK_CHECK_CROSSINGS)

# Metrics shown per run and given a noise band, as (report key, field) pairs.
METRICS = [
    ("main_ms", "median"),
    ("main_ms", "p95"),
    ("main_ms", "p99"),
    ("main_ms", "worst"),
    ("render_ms", "median"),
    ("render_ms", "p95"),
    ("render_ms", "p99"),
    ("render_ms", "worst"),
    ("interval_ms", "p99"),
    ("interval_ms", "worst"),
    ("cpu_ms_per_frame", None),
    ("cpu_s_per_s", None),
    ("peak_rss_mib", None),
]


def parse_report(text: str) -> dict:
    """Parses a `key: value` report; distributions become dicts of floats, traces a list."""
    report: dict = {"trace": []}
    for line in text.splitlines():
        key, separator, value = line.partition(": ")
        if not separator:
            continue
        if key == "trace":
            report["trace"].append(dict(part.split("=", 1) for part in value.split(" ", 5)))
            continue
        if "median=" in value:
            report[key] = {
                name: float(number)
                for name, number in (part.split("=", 1) for part in value.split())
            }
            continue
        report[key] = value
    return report


def metric_value(report: dict, key: str, field: str | None) -> float | None:
    value = report.get(key)
    if field is not None:
        return value.get(field) if isinstance(value, dict) else None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def noise_band(values: list[float]) -> tuple[float, float]:
    """Median across runs and the relative spread (max - min) / median, in percent."""
    middle = statistics.median(values)
    spread = (max(values) - min(values)) / middle * 100.0 if middle else 0.0
    return middle, spread


def label(key: str, field: str | None) -> str:
    return f"{key}.{field}" if field else key


def check_workload(name: str, workload: dict, report: dict, save_dir: Path) -> list[str]:
    """What this run must show to count as having exercised its workload."""
    problems = []
    if report.get("status") != "ok":
        problems.append(f"status: {report.get('status', 'no report')}")
    if report.get("build") != "release":
        problems.append(f"build is {report.get('build')}, not release")
    script_steps = [
        line.split("#", 1)[0].strip()
        for line in workload["script"].splitlines()
        if line.split("#", 1)[0].strip()
    ]
    traced = [entry.get("step") for entry in report["trace"]]
    if traced != script_steps:
        problems.append(f"only {len(traced)} of {len(script_steps)} script steps completed")
    if workload.get("expect_entity_churn"):
        entities = report.get("entities", "")
        counts = dict(part.split("=", 1) for part in entities.split()) if "=" in entities else {}
        if not counts or counts["min"] == counts["max"]:
            problems.append("no effect entities were spawned during the window")
    if "expect_snapshots" in workload:
        rows = read_snapshots(save_dir.parent / "report.snapshots.csv")
        expected = workload["expect_snapshots"]
        if len(rows) != expected:
            problems.append(f"{len(rows)} snapshots were recorded, expected {expected}")
        arrivals = [row["label"] for row in rows if row["label"].startswith("arrive ")]
        wanted = [
            line.split(" ", 1)[1]
            for line in script_steps
            if line.startswith("snapshot arrive ")
        ]
        if arrivals != wanted:
            problems.append("the arrivals recorded are not the arrivals scripted")
    if "expect_autosave" in workload:
        autosave = save_dir / "000.yaml"
        expected = workload["expect_autosave"]
        if expected is None and autosave.exists():
            problems.append("autosave slot 0 was written, but this workload must not autosave")
        if expected is not None:
            if not autosave.exists():
                problems.append("autosave slot 0 was not written")
            elif f"current: {expected}" not in autosave.read_text(encoding="utf-8"):
                problems.append(f"autosave slot 0 does not record arrival on {expected}")
    del name
    return problems


def read_snapshots(path: Path) -> list[dict]:
    """Rows of a `.snapshots.csv`: label plus every numeric column as an int (empty RSS -> None)."""
    if not path.exists():
        return []
    lines = path.read_text(encoding="utf-8").splitlines()
    header = lines[0].split(",")
    rows = []
    for line in lines[1:]:
        values = line.split(",")
        row: dict = {"label": values[0]}
        for name, value in zip(header[1:], values[1:]):
            row[name] = int(value) if value else None
        rows.append(row)
    return rows


def slope(values: list[float]) -> float:
    """Least-squares slope per index."""
    count = len(values)
    if count < 2:
        return 0.0
    mean_x = (count - 1) / 2
    mean_y = sum(values) / count
    numerator = sum((x - mean_x) * (y - mean_y) for x, y in enumerate(values))
    return numerator / sum((x - mean_x) ** 2 for x in range(count))


def snapshot_table(rows: list[dict]) -> list[str]:
    """The per-arrival table, plus the RSS slope per transition over the arrivals."""
    if not rows:
        return []
    columns = [name for name in rows[0] if name not in ("label", "frame")]
    widths = {name: max(len(name), 7) for name in columns}
    label_width = max(len(row["label"]) for row in rows)
    lines = [
        f"{'label':<{label_width}}"
        + "".join(f" {name:>{widths[name]}}" for name in columns)
    ]
    for row in rows:
        lines.append(
            f"{row['label']:<{label_width}}"
            + "".join(
                f" {'n/a' if row[name] is None else row[name]:>{widths[name]}}" for name in columns
            )
        )
    rss = [row["rss_kib"] for row in rows if row["rss_kib"] is not None]
    if len(rss) >= 2:
        lines.append(f"rss slope: {slope(rss):.1f} KiB per snapshot over {len(rss)} snapshots")
    return lines


def command_output(command: list[str]) -> str:
    try:
        return subprocess.run(
            command, cwd=REPO, capture_output=True, text=True, check=True
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def cpu_model() -> str:
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "unknown"


def load_average() -> str:
    try:
        return " ".join(Path("/proc/loadavg").read_text(encoding="utf-8").split()[:3])
    except OSError:
        return "unknown"


def run_once(name: str, workload: dict, run_dir: Path, seed: int, binary: Path) -> dict:
    save_dir = run_dir / "save"
    config_dir = run_dir / "config"
    save_dir.mkdir(parents=True)
    config_dir.mkdir()
    if workload.get("save_fixture"):
        shutil.copyfile(SAVE_FIXTURE, save_dir / "001.yaml")
    script = run_dir / f"{name}.script"
    script.write_text(workload["script"], encoding="utf-8")
    report_path = run_dir / "report.txt"
    environment = dict(os.environ)
    # xvfb-run supplies DISPLAY; never let winit reach a real X or Wayland session instead.
    for variable in ("DISPLAY", "WAYLAND_DISPLAY", "WAYLAND_SOCKET"):
        environment.pop(variable, None)
    environment.update(
        RPG_S1_MUTE_AUDIO="1",
        RPG_S1_SAVE_DIR=str(save_dir),
        RPG_S1_CONFIG_DIR=str(config_dir),
    )
    environment.pop("RPG_S1_PARTY_PRESET", None)
    load = load_average()
    command = [
        "timeout",
        str(workload.get("timeout_s", RUN_TIMEOUT_SECONDS)),
        "xvfb-run",
        "-a",
        "-s",
        XVFB_SCREEN,
        str(binary),
        "play",
        *workload["args"],
        "--seed",
        str(seed),
        "--perf-script",
        str(script),
        "--perf-report",
        str(report_path),
    ]
    with open(run_dir / "game.log", "w", encoding="utf-8") as log:
        completed = subprocess.run(
            command, cwd=REPO, env=environment, stdout=log, stderr=subprocess.STDOUT
        )
    report = (
        parse_report(report_path.read_text(encoding="utf-8"))
        if report_path.exists()
        else {"trace": [], "status": f"no report (exit {completed.returncode})"}
    )
    report["load_average"] = load
    report["problems"] = check_workload(name, workload, report, save_dir)
    return report


def run_workload(name: str, runs: int, seed: int, out: Path, binary: Path) -> bool:
    workload = WORKLOADS[name]
    out.mkdir(parents=True, exist_ok=True)
    reports = []
    for index in range(1, runs + 1):
        report = run_once(name, workload, out / f"run-{index}", seed, binary)
        reports.append(report)
    text = summarize(name, workload, reports, seed, out)
    (out / "summary.txt").write_text(text, encoding="utf-8")
    print(text, end="")
    return all(not report["problems"] for report in reports)


def summarize(name: str, workload: dict, reports: list[dict], seed: int, out: Path) -> str:
    first = reports[0]
    dirty = command_output(["git", "status", "--porcelain", "--untracked-files=no"])
    lines = [
        f"workload: {name} - {workload['about']}",
        f"commit: {command_output(['git', 'rev-parse', '--short', 'HEAD'])}"
        + (" (working tree has uncommitted changes)" if dirty not in ("", "unknown") else ""),
        f"build: {first.get('build', 'unknown')}",
        f"cpu: {cpu_model()} ({os.cpu_count()} logical cpus)",
        f"kernel: {command_output(['uname', '-sr'])}",
        f"adapter: {first.get('adapter', 'unknown')}",
        f"pacing: {first.get('pacing', 'unknown')}",
        f"seed: {seed}",
        f"frames: {first.get('frames', 'unknown')}",
        f"raw data: {out}",
        "",
    ]
    metrics = list(METRICS) + [(key, None) for key in workload.get("metrics_extra", [])]
    header = f"{'metric':<22}" + "".join(f"{f'run {i + 1}':>11}" for i in range(len(reports)))
    if len(reports) > 1:
        header += f"{'median':>11}{'spread%':>9}"
    lines.append(header)
    for key, field in metrics:
        values = [metric_value(report, key, field) for report in reports]
        row = f"{label(key, field):<22}" + "".join(
            f"{value:>11.3f}" if value is not None else f"{'n/a':>11}" for value in values
        )
        present = [value for value in values if value is not None]
        if len(reports) > 1 and len(present) == len(values):
            middle, spread = noise_band(present)
            row += f"{middle:>11.3f}{spread:>8.1f}%"
        lines.append(row)
    lines.append(
        f"{'load avg at start':<22}"
        + "".join(f"{report['load_average'].split()[0]:>11}" for report in reports)
    )
    lines.append("")
    if workload.get("expect_snapshots"):
        for index in range(1, len(reports) + 1):
            lines.append("")
            lines.append(f"run {index} snapshots:")
            lines += snapshot_table(read_snapshots(out / f"run-{index}" / "report.snapshots.csv"))
        lines.append("")
    for index, report in enumerate(reports, start=1):
        verdict = "verified" if not report["problems"] else "FAILED: " + "; ".join(report["problems"])
        lines.append(f"run {index}: {verdict}")
    return "\n".join(lines) + "\n"


def default_out(name: str) -> Path:
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M")
    return REPO / "temp" / f"{stamp}-perf-{name}"


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("workload", nargs="?", choices=[*WORKLOADS, "all"])
    parser.add_argument("--runs", type=int, default=1, help="repetitions; 3+ gives a noise band")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--out", type=Path, help="raw data directory (default temp/YYYYMMDD-HHMM-perf-WORKLOAD)")
    parser.add_argument("--no-build", action="store_true", help="skip `cargo build --release`")
    parser.add_argument("--list", action="store_true", help="list the workloads")
    arguments = parser.parse_args(argv)
    if arguments.list or arguments.workload is None:
        for name, workload in WORKLOADS.items():
            print(f"{name:<11} {workload['about']}")
        return 0
    if arguments.runs < 1:
        parser.error("--runs must be at least 1")
    for tool in ("xvfb-run", "timeout"):
        if shutil.which(tool) is None:
            print(f"error: `{tool}` is required", file=sys.stderr)
            return 2
    if not arguments.no_build:
        subprocess.run(["cargo", "build", "--release"], cwd=REPO, check=True)
    if not BINARY.exists():
        print(f"error: {BINARY} is missing; build with `cargo build --release`", file=sys.stderr)
        return 2
    names = list(WORKLOADS) if arguments.workload == "all" else [arguments.workload]
    out = arguments.out or default_out(arguments.workload)
    ok = True
    for name in names:
        target = out / name if arguments.workload == "all" else out
        ok = run_workload(name, arguments.runs, arguments.seed, target, BINARY) and ok
        print()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
