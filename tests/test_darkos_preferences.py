"""The selector preparation is purely in-memory; no device settings are written."""
import importlib.util
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

spec = importlib.util.spec_from_file_location('preferences', Path(__file__).resolve().parents[1]/'tools/darkos_preferences.py')
preferences = importlib.util.module_from_spec(spec)
spec.loader.exec_module(preferences)


class PreferencePreparation(unittest.TestCase):
    def change(self, data, system='snes', emulator='retroarch', core='snes9x'):
        return preferences.prepare_emulator_preferences(data, system, emulator, core)

    def test_fragment_changes_only_the_two_values_and_retains_every_other_byte(self):
        raw = (b'<?xml version="1.0"?>\n<!-- preserve -->\n'
               b'<string name="snes.emulator" note=" value=\'ignore\' >" value=\'old\'/>\n'
               b'<string value="older" name="snes.core"/>\n<bool name="other" value="true"/>\n')
        expected = raw.replace(b"value='old'", b"value='retroarch'").replace(b'value="older"', b'value="snes9x"')
        self.assertEqual(self.change(raw), expected)

    def test_missing_entries_are_inserted_into_fragment_and_wrapped_config(self):
        for raw in (b'<?xml version="1.0"?>\n<bool name="x" value="true"/>\n',
                    b'<?xml version="1.0"?>\n<config>\n<bool name="x" value="true"/>\n</config>\n'):
            result = self.change(raw, system='Sega & Saturn')
            body = result.split(b'?>', 1)[1]
            root = ET.fromstring(b'<outer>'+body+b'</outer>')
            values = {e.get('name'):e.get('value') for e in root.iter('string')}
            self.assertEqual(values, {'Sega & Saturn.emulator':'retroarch', 'Sega & Saturn.core':'snes9x'})
            self.assertIn(b'<bool name="x" value="true"/>', result)

    def test_duplicate_nested_dtd_malformed_and_unsafe_inputs_are_refused(self):
        for raw in (b'<string name="snes.core" value="a"/><string name="snes.core" value="b"/>',
                    b'<other><string name="snes.core" value="a"/></other>',
                    b'<!DOCTYPE x><string name="snes.core" value="a"/>',
                    b'<string name="snes.core" value="a"', b'<config/><bool name="x"/>'):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                self.change(raw)
        with self.assertRaises(ValueError): self.change(b'', core='../invalid')
        with self.assertRaises(ValueError): self.change(b'', system='bad\nname')

    def test_self_closing_wrapper_fails_closed_when_insertion_is_needed(self):
        with self.assertRaises(ValueError): self.change(b'<config/>')


if __name__ == '__main__':
    unittest.main()
