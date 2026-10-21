#!/usr/bin/env python3
"""Three bounded controller replays against the real Slint simulator.

The fixture seed and fixture directory are read only. Each run uses a private
SQLite copy; these are synthetic own fixtures, never a user's library or ROMs.
No hardware, network, screenshots, build or performance claims are involved.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time


BUTTONS = {
    "up": 0, "down": 1, "left": 2, "right": 3,
    "confirm": 4, "back": 5, "previous": 6, "next": 7,
    "details": 8, "menu": 9, "library": 10,
}


class Replay:
    def __init__(self, name, coverage):
        self.name = name
        self.coverage = coverage
        self.steps = []
        self.at = 800
        self.check(at=700, active_section=0, selected=0, resume_count=3,
                   has_recent=True, game_options=False, audio_open=False,
                   system_open=False, menu=False, library=False)

    def check(self, *, at=None, **expected):
        self.steps.append(dict(at_ms=self.at if at is None else at, **expected))

    def press(self, action, *, at=None, **expected):
        at = self.at if at is None else at
        code = BUTTONS[action]
        self.steps.extend([
            dict(at_ms=at, kind=1, code=code, value=1, **expected),
            dict(at_ms=at + 8, kind=1, code=code, value=0, held=False),
        ])
        self.at = at + 100

    def finish(self):
        self.check(at=6000, game_options=False, audio_open=False,
                   system_open=False, held=False)
        assert len(self.steps) < 5000
        assert all(a["at_ms"] <= b["at_ms"] for a, b in zip(self.steps, self.steps[1:]))
        return self

    def write(self, path):
        lines = ["keyboard=false", "[mapping]"]
        for action, code in BUTTONS.items():
            lines.extend(["[[mapping.buttons]]", f"code={code}",
                          "action=" + json.dumps(action)])
        for step in self.steps:
            lines.append("[[steps]]")
            lines.extend(key + "=" + json.dumps(value) for key, value in step.items())
        path.write_text("\n".join(lines) + "\n")


def cases():
    cycle = Replay("cycle-focus", [
        "Forward and reverse four-section circular order",
        "Three distinct rapid shoulder presses produce exactly three changes",
        "Independent primary/alternative, library and settings focus",
        "Top tabs separate active destination from focused cursor",
        "Tabs-content-capsule return route",
    ])
    cycle.press("down", selected=4)
    cycle.press("right", selected=5)
    cycle.press("next", active_section=1, selected=0, section_title="Tu momento")
    cycle.press("down", selected=1)
    cycle.press("next", active_section=2, library=True, library_index=0)
    cycle.press("down", library_index=1)
    cycle.press("next", active_section=3, menu=True, menu_index=0)
    cycle.press("down", menu_index=1)
    for section, expected in [(0, {"selected": 5}), (1, {"selected": 1}),
                              (2, {"library_index": 1}), (3, {"menu_index": 1}),
                              (0, {"selected": 5})]:
        cycle.press("next", active_section=section, **expected)
    for section, expected in [(3, {"menu_index": 1}), (2, {"library_index": 1}),
                              (1, {"selected": 1}), (0, {"selected": 5})]:
        cycle.press("previous", active_section=section, **expected)
    for at, section in [(2600, 1), (2620, 2), (2640, 3)]:
        cycle.press("next", at=at, active_section=section)
    for at, section in [(2800, 2), (2820, 1), (2840, 0)]:
        cycle.press("previous", at=at, active_section=section)
    cycle.press("up", at=3200, selected=0)
    cycle.press("up", dock_cursor=0, active_section=0)
    cycle.press("right", dock_cursor=1, active_section=0)
    cycle.press("confirm", dock_cursor=1, active_section=1)
    cycle.press("down", dock_cursor=-1, selected=1)
    cycle.press("up", selected=0)
    cycle.press("up", dock_cursor=1)
    cycle.press("up", dock_cursor=-1, system_focused=True)
    cycle.press("back", dock_cursor=1, system_focused=False, active_section=1)
    cycle.press("down", dock_cursor=-1, selected=0)
    cycle.press("up", dock_cursor=1)
    cycle.press("right", dock_cursor=2)
    cycle.press("right", dock_cursor=3)
    cycle.press("right", dock_cursor=-1, system_focused=True)
    cycle.press("confirm", system_open=True, active_section=1)
    cycle.press("back", system_open=False, system_focused=True)
    cycle.press("left", system_focused=False, dock_cursor=3)

    game = Replay("game-modal", [
        "Contextual modal owns input exclusively",
        "L/R and Library are consumed while a modal is open",
        "B restores a secondary card without propagating",
        "Quick reopen/close has no stale overlay",
        "B closes a library game menu without also leaving Library",
    ])
    game.press("down", selected=4)
    game.press("details", game_options=True, game_option_index=0,
               audio_open=False, system_open=False, selected=4)
    for action in ["next", "previous", "library"]:
        game.press(action, active_section=0, game_options=True,
                   selected=4, menu=False, library=False)
    game.press("down", game_option_index=1)
    game.press("next", active_section=0, game_options=True, game_option_index=1)
    game.press("back", game_options=False, selected=4, active_section=0,
               library=False, menu=False, audio_open=False, system_open=False)
    game.press("details", at=1520, game_options=True, game_option_index=0)
    game.press("back", at=1540, game_options=False, selected=4, active_section=0)
    game.press("next", at=1800, active_section=1, selected=0)
    game.press("details", game_options=True, audio_open=False, system_open=False)
    game.press("back", game_options=False, selected=0, active_section=1)
    game.press("next", active_section=2, library=True, library_index=0)
    game.press("down", library_index=1)
    game.press("details", game_options=True, library=True, library_index=1)
    game.press("next", active_section=2, game_options=True, library_index=1)
    game.press("back", game_options=False, library=True, library_index=1, active_section=2)
    game.press("back", library=False, active_section=1, selected=0)

    panels = Replay("audio-capsule", [
        "Audio modal blocks L/R/Library and restores Settings focus with B",
        "Another modal shortcut cannot open behind audio",
        "Capsule retains input until B and restores its previous focus",
        "Closing the capsule does not also close Settings",
        "Capsule's explicit Settings action closes before navigation",
    ])
    panels.press("previous", active_section=3, menu=True, menu_index=0)
    panels.press("down", menu_index=1)
    panels.press("confirm", audio_open=True, game_options=False, system_open=False)
    for action in ["next", "previous", "library", "details"]:
        panels.press(action, active_section=3, audio_open=True,
                     game_options=False, system_open=False, menu_index=1)
    panels.press("back", audio_open=False, menu=True, menu_index=1, active_section=3)
    panels.press("confirm", at=1700, audio_open=True)
    panels.press("back", at=1720, audio_open=False, menu_index=1, active_section=3)
    panels.press("up", at=2000, menu_index=0)
    panels.press("up", dock_cursor=3, system_focused=False)
    panels.press("up", dock_cursor=-1, system_focused=True)
    panels.press("confirm", system_open=True, system_focused=True,
                 audio_open=False, game_options=False)
    for action in ["next", "previous", "down", "details"]:
        panels.press(action, system_open=True, active_section=3,
                     audio_open=False, game_options=False)
    panels.press("back", system_open=False, system_focused=True, active_section=3,
                 menu=True, menu_index=0)
    panels.press("back", system_focused=False, dock_cursor=3, active_section=3)
    panels.press("down", dock_cursor=-1, menu_index=0, active_section=3)
    panels.press("up", dock_cursor=3)
    panels.press("up", system_focused=True)
    panels.press("confirm", system_open=True)
    panels.press("confirm", system_open=False, audio_open=False, game_options=False,
                 active_section=3, dock_cursor=3)
    return [cycle.finish(), game.finish(), panels.finish()]


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--seed", type=Path,
                        help="Defaults to OUT/../visual-seed.sqlite")
    parser.add_argument("--fixture-root", type=Path,
                        help="Defaults to SEED/../fixtures")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    out = args.out.resolve()
    seed = (args.seed or out.parent / "visual-seed.sqlite").resolve(strict=True)
    fixtures = (args.fixture_root or seed.parent / "fixtures").resolve(strict=True)
    if not (fixtures / "snes/gamelist.xml").is_file():
        parser.error("Expected the existing owned three-game SNES fixture")
    # Opening the seed is unnecessary: copy bytes to an isolated writable DB.
    # This also avoids creating WAL sidecars alongside a read-only source.
    seed_hash = digest(seed)
    out.mkdir(parents=True, exist_ok=False)
    out.chmod(0o700)
    config = out / "context.toml"
    config.write_text('at="2026-12-25T16:00"\nweather="snow"\n'
                      '[ambient]\ntimezone="Etc/UTC"\nhemisphere="north"\n')
    report = dict(binary_sha256=digest(binary), seed_sha256=seed_hash,
                  physical_verified=False, performance_measured=False,
                  scope="Slint software simulator controller replays; fixed synthetic own library",
                  cases=[])
    env = dict(os.environ, SLINT_SCALE_FACTOR="1",
               EMOTIONAL_ARTWORK_CACHE=str(out / "artwork-cache"))
    for name in ["SLINT_DEBUG_PERFORMANCE", "EMOTIONAL_PERF", "EMOTIONAL_PERF_RESOURCES",
                 "EMOTIONAL_PERF_TARGET", "EMOTIONAL_RENDER_LAYERS"]:
        env.pop(name, None)
    for case in cases():
        path = out / (case.name + ".toml")
        case.write(path)
        database = out / (case.name + ".sqlite")
        shutil.copyfile(seed, database)
        command = [str(binary), "--db", str(database), "--library-root", str(fixtures),
                   "--view", "resume", "--config", str(config), "--sim-replay", str(path),
                   "--audio-mode", "silence", "--audio-volume", "0",
                   "--smoke-test", "--smoke-seconds", "8"]
        started = time.monotonic()
        try:
            child = subprocess.run(command, env=env, capture_output=True, text=True, timeout=18)
            stdout, stderr, code = child.stdout, child.stderr, child.returncode
        except subprocess.TimeoutExpired as error:
            stdout = error.stdout or b""
            stderr = error.stderr or b""
            stdout = stdout.decode(errors="replace") if isinstance(stdout, bytes) else stdout
            stderr = stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr
            stderr += "\nReplay exceeded its bounded runtime.\n"
            code = None
        (out / (case.name + ".stdout")).write_text(stdout)
        (out / (case.name + ".stderr")).write_text(stderr)
        marker = f"SIMULATOR: {len(case.steps)} raw-input replay steps passed; physical_input_verified=false"
        passed = code == 0 and marker in stdout
        row = dict(name=case.name, passed=passed, returncode=code, steps=len(case.steps),
                   elapsed_seconds=round(time.monotonic() - started, 3), coverage=case.coverage)
        report["cases"].append(row)
        print(case.name + ": " + ("PASS" if passed else "FAIL"), flush=True)
    report["seed_unchanged"] = digest(seed) == seed_hash
    report["passed"] = report["seed_unchanged"] and all(row["passed"] for row in report["cases"])
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
