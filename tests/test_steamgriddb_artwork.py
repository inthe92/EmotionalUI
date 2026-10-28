"""Offline provider contract, exact identities, unchanged inputs and real image decoding."""
import copy
import importlib.util
import io
import json
import sqlite3
import tempfile
import tomllib
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import patch

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("sgdb", ROOT / "tools/steamgriddb_artwork.py")
sgdb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sgdb)


def image_bytes(size=(600, 900), format="PNG"):
    image = Image.new("RGB", size, (50, 100, 180))
    stream = io.BytesIO()
    image.save(stream, format=format)
    return stream.getvalue()


def grid(grid_id=10, **fields):
    return {"id": grid_id, "width": 600, "height": 900, "score": 5,
            "url": f"https://cdn2.steamgriddb.com/grid/{grid_id}.png",
            "author": {"name": "Fixture artist"}, **fields}


class FakeClient:
    def __init__(self, grids=None):
        self.results = [grid()] if grids is None else grids
        self.searches, self.grid_calls, self.image_calls = [], [], []

    def search(self, title):
        self.searches.append(title)
        return [{"id": 123, "name": title, "release_date": 100, "types": [], "verified": True},
                {"id": 456, "name": title + " Remake"}]

    def grids(self, game_id):
        self.grid_calls.append(game_id)
        return self.results

    def image(self, url):
        self.image_calls.append(url)
        return image_bytes()


class SteamGridDBArtwork(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.cache = self.root / "cache"
        self.inventory = {"version": 1, "games": [
            {"game_id": 'es:/roms2/snes/a "title".sfc', "system": "snes", "title": "Same title"},
            {"game_id": "es:/roms2/megadrive/a.md", "system": "megadrive", "title": "Same title"}]}
        self.plan = sgdb.proposals(self.inventory, FakeClient())

    def tearDown(self):
        self.temp.cleanup()

    def approved(self):
        plan = copy.deepcopy(self.plan)
        for row in plan["matches"]:
            row.update(approved=True, sgdb_id=123)
        return plan

    def test_inventory_is_read_only_and_preserves_exact_id_and_system(self):
        db = self.root / "library.sqlite"
        with sqlite3.connect(db) as connection:
            connection.execute("CREATE TABLE imported_games(id,source_xml,metadata)")
            connection.execute("INSERT INTO imported_games VALUES(?,?,?)", (
                "es:/roms2/snes/not-renamed.sfc", "/roms2/snes/gamelist.xml",
                'name="My game"\nboxart="original.png"'))
        before = db.read_bytes()
        result = sgdb.inventory(db)
        self.assertEqual(result["games"][0], {"game_id": "es:/roms2/snes/not-renamed.sfc", "system": "snes", "title": "My game"})
        self.assertEqual(db.read_bytes(), before)
        self.assertIn("not checked", result["availability"])

    def test_same_title_search_is_cached_but_never_assigns_first_result(self):
        client = FakeClient()
        plan = sgdb.proposals(self.inventory, client)
        self.assertEqual(client.searches, ["Same title"])
        self.assertEqual(len(plan["matches"]), 2)
        self.assertTrue(all(not row["approved"] and row["sgdb_id"] is None for row in plan["matches"]))
        with self.assertRaisesRegex(ValueError, "No reviewed"):
            sgdb.approved_matches(self.inventory, plan)
        self.assertEqual(sgdb.proposals(self.inventory, client, "snes")["matches"][0]["system"], "snes")
        self.assertEqual(len(sgdb.proposals(self.inventory, client, offset=1)["matches"]), 1)

    def test_changed_system_id_title_or_inventory_never_applies_mapping(self):
        for key, value in [("system", "nes"), ("game_id", "other"), ("title", "Remake"), ("sgdb_id", True)]:
            plan = self.approved()
            plan["matches"][0][key] = value
            with self.assertRaises(ValueError):
                sgdb.approved_matches(self.inventory, plan)
        modified = copy.deepcopy(self.inventory)
        modified["games"][0]["title"] = "New title"
        with self.assertRaisesRegex(ValueError, "snapshot"):
            sgdb.approved_matches(modified, self.approved())

    def test_png_and_jpeg_normalize_without_cropping_or_stretching(self):
        for format in ("PNG", "JPEG"):
            master = image_bytes(format=format)
            result = sgdb.portrait(master)
            self.assertEqual(Image.open(io.BytesIO(result)).size, (240, 360))
            self.assertEqual(Image.open(io.BytesIO(master)).size, (600, 900))
        for raw in (image_bytes((900, 600)), image_bytes(format="WEBP"), b"corrupt", b"x" * (sgdb.MAX_IMAGE + 1)):
            with self.assertRaises(ValueError):
                sgdb.portrait(raw)

    def test_animation_is_rejected_even_if_named_png(self):
        output = io.BytesIO()
        first = Image.new("RGB", (600, 900), "blue")
        first.save(output, format="PNG", save_all=True, append_images=[Image.new("RGB", (600, 900), "red")], duration=100)
        with self.assertRaisesRegex(ValueError, "static"):
            sgdb.portrait(output.getvalue())

    def test_grid_filter_and_stable_choice(self):
        self.assertEqual(sgdb.pick_grid([grid(12), grid(10)])["id"], 10)
        self.assertEqual(sgdb.pick_grid([grid(12), grid(10)], 12)["id"], 12)
        for invalid in [grid(width=900, height=600), grid(animated=True), grid(nsfw=True), grid(humor=True),
                        grid(epilepsy=True), grid(url="https://evil.test/grid/1.png")]:
            self.assertIsNone(sgdb.pick_grid([invalid]))
        self.assertIsNone(sgdb.pick_grid([grid()], 999))

    def test_asset_requests_never_receive_bearer_and_api_uses_official_filters(self):
        calls = []
        def network(url, limit, headers=None):
            calls.append((url, headers))
            return b'{"success":true,"data":[]}' if "/api/v2/" in url else image_bytes()
        with patch.object(sgdb, "download", side_effect=network):
            client = sgdb.Client("private-test-token")
            client.grids(123)
            client.image(grid()["url"])
        self.assertIn("/grids/game/123?", calls[0][0])
        self.assertIn("dimensions=600x900", calls[0][0])
        self.assertIn("types=static", calls[0][0])
        self.assertEqual(calls[0][1], {"Authorization": "Bearer private-test-token"})
        self.assertIsNone(calls[1][1])
        self.assertIsNone(sgdb.NoRedirect().redirect_request(None, None, 302, "", {}, "https://evil.test"))

    def test_reject_remote_urls_before_network_access(self):
        for url in ["http://cdn2.steamgriddb.com/grid/1.png", "https://cdn2.steamgriddb.com.evil.test/grid/a.png",
                    "https://private@cdn2.steamgriddb.com/grid/a.png", "file:///grid/a.png",
                    "https://127.0.0.1/grid/a.png", "https://cdn2.steamgriddb.com:444/grid/a.png",
                    "https://cdn2.steamgriddb.com/grid/a.png?token=secret"]:
            with self.assertRaises(ValueError):
                sgdb.Client("secret").image(url)

    def test_fetch_reuses_verified_cache_deduplicates_systems_and_retains_provenance(self):
        client = FakeClient()
        result = sgdb.fetch(self.inventory, self.approved(), client, self.cache)
        self.assertEqual(client.grid_calls, [123])
        self.assertEqual(len(client.image_calls), 1)
        self.assertEqual(len(result["games"]), 2)
        self.assertEqual(result["games"][0]["author"], "Fixture artist")
        self.assertEqual(result["games"][0]["source_page"], "https://www.steamgriddb.com/grid/10")
        with patch.object(client, "grids", side_effect=AssertionError("cache should be offline")):
            self.assertEqual(sgdb.fetch(self.inventory, self.approved(), client, self.cache), result)
        derivative = self.cache / result["games"][0]["portrait"]["path"]
        derivative.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "checksum"):
            sgdb.fetch(self.inventory, self.approved(), client, self.cache)

    def test_missing_portrait_exports_empty_overrides_preserving_original_fallback(self):
        result = sgdb.fetch(self.inventory, self.approved(), FakeClient([]), self.cache)
        self.assertEqual(result["games"], [])
        self.assertEqual(len(result["missing_portrait"]), 2)
        self.assertEqual(tomllib.loads(sgdb.export_overrides(result, self.cache, "/own/art").decode()), {"games": {}})

    def test_export_uses_exact_ids_portable_root_and_checked_portraits(self):
        result = sgdb.fetch(self.inventory, self.approved(), FakeClient(), self.cache)
        before = sgdb.canonical(self.inventory)
        output = sgdb.export_overrides(result, self.cache, "/home/ark/own-art")
        overrides = tomllib.loads(output.decode())["games"]
        self.assertEqual(set(overrides), {row["game_id"] for row in self.inventory["games"]})
        for art in overrides.values():
            self.assertEqual(art["root"], "/home/ark/own-art")
            self.assertTrue(art["path"].startswith("/home/ark/own-art/portraits/"))
        self.assertEqual(before, sgdb.canonical(self.inventory))
        for root in ("/", "relative", "/own/../etc"):
            with self.assertRaises(ValueError):
                sgdb.export_overrides(result, self.cache, root)
        result["games"][0]["portrait"]["path"] = "../escape.png"
        with self.assertRaises(ValueError):
            sgdb.export_overrides(result, self.cache, "/own/art")

    def test_no_key_does_not_start_network_or_output(self):
        with patch.dict(sgdb.os.environ, {}, clear=True), patch.object(sgdb, "KEY_FILE", self.root / "absent"):
            with self.assertRaisesRegex(ValueError, "Configure"):
                sgdb.load_key()
        private = self.root / "secret.key"
        sgdb.atomic_write(private, b"test-key\n")
        self.assertEqual(private.stat().st_mode & 0o777, 0o600)
        with patch.dict(sgdb.os.environ, {}, clear=True), patch.object(sgdb, "KEY_FILE", private):
            self.assertEqual(sgdb.load_key(), "test-key")
            private.chmod(0o644)
            with self.assertRaisesRegex(ValueError, "private"):
                sgdb.load_key()

    def test_output_cannot_replace_catalog_or_mapping_input(self):
        for argv in [
            ["inventory", "--db", str(self.root / "source.json"), "--output", str(self.root / "source.json")],
            ["inventory", "--db", str(self.root / "library.sqlite"), "--output", str(self.root / "game.sfc")],
            ["search", "--inventory", str(self.root / "plan.json"), "--output", str(self.root / "plan.json")]]:
            with self.assertRaisesRegex(ValueError, "separate artwork"):
                sgdb.main(argv)

    def test_resumed_cache_rejects_changed_provider_identity(self):
        sgdb.fetch(self.inventory, self.approved(), FakeClient(), self.cache)
        journal = next((self.cache / "selections").glob("*.json"))
        record = sgdb.read_json(journal)
        record["sgdb_id"] = 999
        sgdb.save_json(journal, record)
        with self.assertRaisesRegex(ValueError, "identity"):
            sgdb.fetch(self.inventory, self.approved(), FakeClient(), self.cache)


if __name__ == "__main__":
    unittest.main()
