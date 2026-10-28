#!/usr/bin/env python3
"""External portrait-art preparation. Never writes a ROM, gamelist or library DB.

Python 3.11+, Pillow. Network/authentication stay outside the frontend.
Search produces proposals, never implicit game/edition matches.
"""
from __future__ import annotations

import argparse
import getpass
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import sqlite3
import sys
import tempfile
import time
try:
    import tomllib
except ImportError:  # The embedded stdlib-only API client also runs on dArkOS Python 3.10.
    tomllib = None
import urllib.error
import urllib.parse
import urllib.request

API = "https://www.steamgriddb.com/api/v2"
KEY_FILE = Path.home() / ".config/emotional-ui/steamgriddb.key"
MAX_IMAGE = 8 * 1024 * 1024
MAX_JSON = 32 * 1024 * 1024  # Local provenance for a multi-system library.
MAX_API_JSON = 4 * 1024 * 1024
MAX_OVERRIDES = 4 * 1024 * 1024
CDN_HOSTS = {"cdn2.steamgriddb.com", "cdn.steamgriddb.com"}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()


def read_json(path):
    with Path(path).open("rb") as file:
        data = file.read(MAX_JSON + 1)
    if len(data) > MAX_JSON:
        raise ValueError("Local artwork JSON exceeds 32 MiB")
    return json.loads(data)


def atomic_write(path, data):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=".sgdb-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as file:
            file.write(data)
            file.flush()
            os.fsync(file.fileno())
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def save_json(path, value):
    atomic_write(path, json.dumps(value, ensure_ascii=False, indent=2).encode() + b"\n")


def inventory(db):
    """Read only EmotionalUI's own DB. No ROM access or filename inference."""
    uri = Path(db).resolve().as_uri() + "?mode=ro"
    connection = sqlite3.connect(uri, uri=True)
    try:
        connection.execute("PRAGMA query_only=ON")
        games = []
        for game_id, source, metadata in connection.execute(
                "SELECT id,source_xml,metadata FROM imported_games ORDER BY id"):
            fields = tomllib.loads(metadata)
            system = fields.get("system") or PurePosixPath(source).parent.name
            title = " ".join(fields.get("name", "").split())
            if not title or not system:
                continue
            games.append({"game_id": game_id, "system": system, "title": title})
        return {"version": 1, "availability": "DB snapshot; files not checked", "games": games}
    finally:
        connection.close()


def checked_inventory(value):
    if value.get("version") != 1 or not isinstance(value.get("games"), list):
        raise ValueError("Invalid inventory")
    rows = {}
    for row in value["games"]:
        if not all(isinstance(row.get(k), str) and row[k] and len(row[k]) <= 4096
                   for k in ("game_id", "system", "title")):
            raise ValueError("Invalid game identity")
        if row["game_id"] in rows:
            raise ValueError("Duplicate game identity")
        rows[row["game_id"]] = row
    return rows


def load_key():
    key = os.environ.get("STEAMGRIDDB_API_KEY", "").strip()
    if not key and KEY_FILE.is_file():
        if KEY_FILE.stat().st_mode & 0o077:
            raise ValueError("Key file must be private (chmod 600)")
        with KEY_FILE.open() as file:
            key = file.read(1025).strip()
    if not key or len(key) > 1024 or not key.isascii() or any(c.isspace() for c in key):
        raise ValueError("Configure the SteamGridDB API key with the configure command")
    return key


class NoRedirect(urllib.request.HTTPRedirectHandler):
    # An Authorization header must never follow an API redirect to another host.
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def download(url, limit, headers=None):
    request = urllib.request.Request(url, headers={"User-Agent": "EmotionalUI-Artwork/1", **(headers or {})})
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=25) as response:
            data = response.read(limit + 1)
    except urllib.error.HTTPError as error:
        # Do not echo response bodies, headers, tokens or signed URLs.
        raise ValueError(f"SteamGridDB HTTP {error.code}; retry later for 429/5xx") from None
    except (urllib.error.URLError, TimeoutError, OSError):
        raise ValueError("SteamGridDB connection failed; retry later") from None
    if len(data) > limit:
        raise ValueError("SteamGridDB response exceeds size limit")
    return data


class Client:
    def __init__(self, key):
        self.key = key
        self.last_request = 0.0

    def api(self, path, **params):
        # Sequential, at most two requests per second; stop on rate-limit errors.
        delay = 0.5 - (time.monotonic() - self.last_request)
        if delay > 0:
            time.sleep(delay)
        url = API + path
        if params:
            url += "?" + urllib.parse.urlencode(params)
        try:
            payload = json.loads(download(url, MAX_API_JSON, {"Authorization": "Bearer " + self.key}))
        finally:
            self.last_request = time.monotonic()
        if payload.get("success") is not True or "data" not in payload:
            raise ValueError("Invalid SteamGridDB API response")
        return payload["data"]

    def search(self, title):
        return self.api("/search/autocomplete/" + urllib.parse.quote(title, safe=""))

    def grids(self, game_id):
        return self.api(f"/grids/game/{positive_id(game_id)}", dimensions="600x900",
                        types="static", mimes="image/png,image/jpeg", nsfw="false",
                        humor="false", epilepsy="false")

    def image(self, url):
        checked_cdn(url)
        # Deliberately no API authentication in CDN requests.
        return download(url, MAX_IMAGE)


def positive_id(value):
    if type(value) is not int or value <= 0:
        raise ValueError("Expected a positive SteamGridDB numeric ID")
    return value


def checked_cdn(url):
    if not isinstance(url, str):
        raise ValueError("Invalid artwork URL")
    parts = urllib.parse.urlsplit(url)
    if (parts.scheme != "https" or parts.hostname not in CDN_HOSTS or parts.port not in (None, 443)
            or parts.username or parts.password or parts.query or parts.fragment
            or not parts.path.startswith("/grid/") or not parts.path.lower().endswith((".png", ".jpg", ".jpeg"))):
        raise ValueError("Artwork must be a static SteamGridDB CDN grid")
    return url


def portrait(data):
    """Lossless master retained separately; output is always static 240x360 PNG."""
    from PIL import Image, ImageOps, UnidentifiedImageError
    if len(data) > MAX_IMAGE:
        raise ValueError("Artwork exceeds 8 MiB")
    try:
        with Image.open(io.BytesIO(data)) as source:
            if source.format not in ("PNG", "JPEG") or source.size != (600, 900) or getattr(source, "n_frames", 1) != 1:
                raise ValueError("Artwork must be static PNG/JPEG, exactly 600x900")
            source.load()
            image = ImageOps.exif_transpose(source).convert("RGBA")
            if image.size != (600, 900):
                raise ValueError("Rotated artwork is not portrait")
            image = image.resize((240, 360), Image.Resampling.LANCZOS)
            output = io.BytesIO()
            image.save(output, format="PNG")
            return output.getvalue()
    except (UnidentifiedImageError, OSError, Image.DecompressionBombError):
        raise ValueError("Invalid artwork image") from None


def proposals(value, client, system=None, limit=20, offset=0):
    rows = checked_inventory(value)
    if not 1 <= limit <= 100 or offset < 0:
        raise ValueError("Use batches of 1..100 games and a nonnegative offset")
    selected = [row for row in rows.values() if system is None or row["system"] == system]
    if not selected:
        raise ValueError("No games for the selected system")
    matches, searches = [], {}
    for row in selected[offset:offset + limit]:
        title = row["title"]
        if title not in searches:
            results = client.search(title)
            searches[title] = [{k: game.get(k) for k in ("id", "name", "release_date", "types", "verified")}
                               for game in results[:20]]
        matches.append({**row, "approved": False, "sgdb_id": None, "grid_id": None,
                        "candidates": searches[title]})
    return {"version": 1, "inventory_sha256": digest(canonical(value)), "matches": matches}


def approved_matches(value, plan):
    rows = checked_inventory(value)
    if plan.get("version") != 1 or plan.get("inventory_sha256") != digest(canonical(value)):
        raise ValueError("Plan does not match this inventory snapshot")
    selected, seen = [], set()
    for match in plan.get("matches", []):
        if match.get("approved") is not True:
            continue
        game_id = match.get("game_id")
        row = rows.get(game_id)
        if row is None or any(match.get(k) != row[k] for k in ("system", "title")) or game_id in seen:
            raise ValueError("Approved mapping has changed identity/system/title or is duplicated")
        positive_id(match.get("sgdb_id"))
        if match.get("grid_id") is not None:
            positive_id(match["grid_id"])
        selected.append(match)
        seen.add(game_id)
    if not selected:
        raise ValueError("No reviewed mappings; search results are proposals only")
    return selected


def pick_grid(grids, requested=None):
    candidates = []
    for grid in grids:
        if ((grid.get("width"), grid.get("height")) != (600, 900)
                or any(grid.get(key) is True for key in ("nsfw", "humor", "epilepsy"))
                or grid.get("animated") is True):
            continue
        try:
            positive_id(grid.get("id"))
            checked_cdn(grid.get("url"))
        except ValueError:
            continue
        candidates.append(grid)
    if requested is not None:
        candidates = [grid for grid in candidates if grid["id"] == requested]
    if not candidates:
        return None
    return sorted(candidates, key=lambda grid: (-float(grid.get("score", 0)), grid["id"]))[0]


def fetch(value, plan, client, cache):
    selected = approved_matches(value, plan)
    cache = Path(cache)
    records, missing, resolved = [], [], {}
    for match in selected:
        selection = (match["sgdb_id"], match.get("grid_id"))
        if selection not in resolved:
            journal = cache / "selections" / (digest(canonical(selection)) + ".json")
            record = read_json(journal) if journal.is_file() else None
            if record:
                # A resumed run trusts neither cache filenames nor cached bytes.
                if (record.get("provider") != "steamgriddb" or record.get("sgdb_id") != selection[0]
                        or (selection[1] is not None and record.get("grid_id") != selection[1])):
                    raise ValueError("Cached selection identity mismatch")
                for field in ("master", "portrait"):
                    asset = record[field]
                    expected = asset["sha256"]
                    path = cache_asset(cache, asset["path"])
                    if digest(path.read_bytes()) != expected:
                        raise ValueError("Cached artwork checksum mismatch")
            else:
                grid = pick_grid(client.grids(match["sgdb_id"]), match.get("grid_id"))
                if grid is None:
                    missing.append(match["game_id"])
                    continue
                raw = client.image(grid["url"])
                reduced = portrait(raw)
                record = {"provider": "steamgriddb", "sgdb_id": match["sgdb_id"], "grid_id": grid["id"],
                          "url": grid["url"], "author": (grid.get("author") or {}).get("name"),
                          "source_page": f"https://www.steamgriddb.com/grid/{grid['id']}",
                          "master": {"path": "masters/" + digest(raw) + ".image", "sha256": digest(raw)},
                          "portrait": {"path": "portraits/" + digest(reduced) + ".png", "sha256": digest(reduced)}}
                atomic_write(cache / record["master"]["path"], raw)
                atomic_write(cache / record["portrait"]["path"], reduced)
                save_json(journal, record)
            resolved[selection] = record
        records.append({k: match[k] for k in ("game_id", "system", "title")} | resolved[selection])
    return {"version": 1, "inventory_sha256": plan["inventory_sha256"], "size": [240, 360],
            "games": records, "missing_portrait": missing}


def cache_asset(cache, relative):
    path = Path(relative)
    root = Path(cache).resolve()
    candidate = (root / path).resolve()
    if path.is_absolute() or ".." in path.parts or not candidate.is_relative_to(root):
        raise ValueError("Artwork cache path escapes its root")
    return candidate


def export_overrides(manifest, cache, device_root):
    from PIL import Image
    root = PurePosixPath(device_root)
    if not root.is_absolute() or ".." in root.parts or root == PurePosixPath("/"):
        raise ValueError("Use a dedicated absolute device artwork directory")
    if manifest.get("version") != 1 or manifest.get("size") != [240, 360]:
        raise ValueError("Invalid portrait manifest")
    lines, seen = [], set()
    for row in manifest["games"]:
        game_id = row["game_id"]
        if not isinstance(game_id, str) or not game_id or game_id in seen:
            raise ValueError("Invalid or duplicate game ID")
        seen.add(game_id)
        asset = row["portrait"]
        path = cache_asset(cache, asset["path"])
        raw = path.read_bytes()
        if len(raw) > MAX_IMAGE or digest(raw) != asset["sha256"]:
            raise ValueError("Portrait checksum mismatch")
        with Image.open(io.BytesIO(raw)) as image:
            if image.size != (240, 360) or image.format != "PNG" or getattr(image, "n_frames", 1) != 1:
                raise ValueError("Invalid prepared portrait")
        remote = root / PurePosixPath(asset["path"])
        # JSON basic quoted strings are valid TOML here; escape control characters.
        quote = lambda value: json.dumps(str(value), ensure_ascii=False)
        lines.extend(["[games." + quote(game_id) + "]", "path = " + quote(remote), "root = " + quote(root), ""])
    result = ("\n".join(lines) + "\n").encode()
    if len(result) > MAX_OVERRIDES:
        raise ValueError("Override manifest exceeds frontend's 4 MiB limit")
    if not seen:
        result = b"[games]\n"
    tomllib.loads(result.decode())
    return result


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("configure", help="Read API key without echo; save outside the repository")
    inv = sub.add_parser("inventory", help="Export only artwork identity from an own SQLite snapshot")
    inv.add_argument("--db", type=Path, required=True)
    inv.add_argument("--output", type=Path, required=True)
    search = sub.add_parser("search", help="Propose matches; never approve automatically")
    search.add_argument("--inventory", type=Path, required=True)
    search.add_argument("--output", type=Path, required=True)
    search.add_argument("--system")
    search.add_argument("--limit", type=int, default=20)
    search.add_argument("--offset", type=int, default=0)
    pull = sub.add_parser("fetch", help="Download only reviewed exact game mappings")
    pull.add_argument("--inventory", type=Path, required=True)
    pull.add_argument("--plan", type=Path, required=True)
    pull.add_argument("--cache", type=Path, required=True)
    pull.add_argument("--output", type=Path, required=True)
    export = sub.add_parser("export", help="Prepare existing frontend overrides; no installation")
    export.add_argument("--manifest", type=Path, required=True)
    export.add_argument("--cache", type=Path, required=True)
    export.add_argument("--device-root", required=True)
    export.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command != "configure":
        expected_suffix = ".toml" if args.command == "export" else ".json"
        protected = [getattr(args, name, None) for name in ("db", "inventory", "plan", "manifest")]
        if (args.output.suffix != expected_suffix or args.output.resolve() == KEY_FILE.resolve()
                or any(path is not None and path.resolve() == args.output.resolve() for path in protected)):
            raise ValueError("Output must be a separate artwork JSON/TOML, never an input or key file")
    if args.command == "configure":
        if not sys.stdin.isatty():
            raise ValueError("Configure requires a private interactive terminal")
        key = getpass.getpass("SteamGridDB API key (hidden): ").strip()
        if not key or len(key) > 1024 or not key.isascii() or any(c.isspace() for c in key):
            raise ValueError("Invalid API key")
        atomic_write(KEY_FILE, (key + "\n").encode())
        print("Key saved privately outside the project")
    elif args.command == "inventory":
        result = inventory(args.db)
        checked_inventory(result)
        save_json(args.output, result)
        print(f"Inventory: {len(result['games'])} identities; files not checked")
    elif args.command == "search":
        result = proposals(read_json(args.inventory), Client(load_key()), args.system, args.limit, args.offset)
        save_json(args.output, result)
        print(f"Proposals: {len(result['matches'])}; none approved")
    elif args.command == "fetch":
        value, plan = read_json(args.inventory), read_json(args.plan)
        approved_matches(value, plan)
        result = fetch(value, plan, Client(load_key()), args.cache)
        save_json(args.output, result)
        print(f"Prepared: {len(result['games'])}; no portrait: {len(result['missing_portrait'])}")
    else:
        data = export_overrides(read_json(args.manifest), args.cache, args.device_root)
        atomic_write(args.output, data)
        print("Override manifest exported; nothing installed")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, sqlite3.Error) as error:
        # Expected errors contain no API key or remote response body.
        print(f"Artwork preparation stopped: {type(error).__name__}: {error}", file=sys.stderr)
        sys.exit(1)
