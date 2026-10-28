"""End-to-end local HTTP tests with synthetic files and temporary libraries."""
import importlib.util
import json
import errno
import os
import socket
import tempfile
import threading
import unittest
from unittest import mock
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('rom_drop', ROOT/'platform/darkos/rom_drop.py')
drop = importlib.util.module_from_spec(spec)
spec.loader.exec_module(drop)


class DropTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.home = self.root/'home'; self.home.mkdir()
        self.library = self.root/'roms'; self.library.mkdir()
        for n in ('nes','snes','ports'): (self.library/n).mkdir()
        self.systems = self.root/'systems.xml'
        self.systems.write_text('<systemList>' + ''.join(
            '<system><name>'+name+'</name><fullname>'+name.upper()+'</fullname><path>'+str(self.library/name)+'</path><extension>'+ext+'</extension></system>'
            for name,ext in [('nes','.nes .zip .sh'),('snes','.sfc .zip'),('ports','.zip')]) + '</systemList>')
        self.catalog = drop.inventory(self.systems,self.home,self.library)
        self.server = drop.DropServer(('127.0.0.1',0), self.catalog, 'secret-token')
        self.thread = threading.Thread(target=self.server.serve_forever,daemon=True); self.thread.start()
        self.base = 'http://127.0.0.1:'+str(self.server.server_port)
    def tearDown(self):
        self.server.shutdown(); self.server.server_close(); self.thread.join(); self.tmp.cleanup()
    def request(self, path, data=None, token='secret-token', headers=None):
        h = {'X-Emotional-Token':token}; h.update(headers or {})
        req = urllib.request.Request(self.base+path,data=data,headers=h)
        try:
            with urllib.request.urlopen(req,timeout=4) as response:
                return response.status, response.read()
        except urllib.error.HTTPError as error:
            return error.code,error.read()
    def upload(self, name, system, content=b'fixture'):
        query = urllib.parse.urlencode({'system':system,'name':name})
        return self.request('/api/file?'+query,content)[0]
    def test_inventory_is_bounded_and_disambiguates_shared_extensions(self):
        self.assertEqual(len(self.catalog),2)
        self.assertEqual([s['name'] for s in self.catalog if '.zip' in s['extensions']],['NES','SNES'])
        self.assertNotIn('.sh',self.catalog[0]['extensions'])
        self.systems.write_text('<!DOCTYPE systemList><systemList/>')
        with self.assertRaises(ValueError): drop.inventory(self.systems,self.home,self.library)
    def test_only_current_compatible_library_destinations_are_offered(self):
        profile=self.root/'launchers.toml'
        profile.write_text('[[launchers]]\nroot = '+json.dumps(str(self.library/'nes'))+'\nextensions = [".nes"]\n')
        systems=drop.restrict_to_launchers(self.catalog, profile)
        self.assertEqual([(s['name'],s['extensions']) for s in systems],[('NES',['.nes'])])
    def test_local_http_multiple_files_and_existing_file_never_overwritten(self):
        status,body=self.request('/api/systems')
        self.assertEqual(status,200);self.assertEqual(len(json.loads(body)['systems']),2)
        self.assertEqual(self.upload('uno.nes','s0',b'first'),201)
        self.assertEqual(self.upload('dos.zip','s1',b'second'),201)
        self.assertEqual(self.upload('uno.nes','s0',b'replacement'),409)
        self.assertEqual((self.library/'nes/uno.nes').read_bytes(),b'first')
        self.assertEqual((self.library/'snes/dos.zip').read_bytes(),b'second')
        self.assertFalse(list(self.library.rglob('.emotional-drop-*.part')))
    def test_auth_origin_path_extensions_and_symlink_targets_are_rejected(self):
        self.assertEqual(self.request('/api/systems',token='wrong')[0],403)
        self.assertEqual(self.request('/api/systems',headers={'Origin':'http://malicious.invalid'})[0],403)
        self.assertEqual(self.upload('../escape.nes','s0'),400)
        self.assertEqual(self.upload('program.sh','s0'),400)
        self.assertEqual(self.upload('wrong.sfc','s0'),400)
        outside=self.root/'original';outside.write_bytes(b'original')
        (self.library/'nes/link.nes').symlink_to(outside)
        self.assertEqual(self.upload('link.nes','s0'),409)
        self.assertEqual(outside.read_bytes(),b'original')
    def test_landing_page_and_html_do_not_include_secret_or_rom_data(self):
        status,page=self.request('/',token='')
        self.assertEqual(status,200);self.assertIn(b'multiple',page)
        self.assertNotIn(b'secret-token',page)
        self.assertNotIn(str(self.library).encode(),page)

    def test_interrupted_upload_leaves_no_destination_or_partial(self):
        with socket.create_connection(('127.0.0.1',self.server.server_port),timeout=4) as client:
            client.sendall(b'POST /api/file?system=s0&name=interrupted.nes HTTP/1.1\r\nHost: localhost\r\nX-Emotional-Token: secret-token\r\nContent-Length: 100\r\n\r\npartial')
            client.shutdown(socket.SHUT_WR)
            self.assertIn(b'400',client.recv(512))
        self.assertFalse((self.library/'nes/interrupted.nes').exists())
        self.assertFalse(list(self.library.rglob('.emotional-drop-*.part')))

    def test_filesystem_without_links_uses_exclusive_copy_and_preserves_existing(self):
        directory = self.library/'nes'
        (directory/'.part').write_bytes(b'content')
        fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            with mock.patch.object(drop.sys, 'platform', 'darwin'), mock.patch.object(drop.os, 'link', side_effect=OSError(errno.EPERM, 'links unsupported')):
                drop.publish(fd,'.part','new.nes',lambda:False)
                self.assertEqual((directory/'new.nes').read_bytes(), b'content')
                with self.assertRaises(FileExistsError): drop.publish(fd,'.part','new.nes',lambda:False)
                with self.assertRaises(ValueError): drop.publish(fd,'.part','cancel.nes',lambda:True)
                self.assertFalse((directory/'cancel.nes').exists())
                self.assertEqual((directory/'new.nes').read_bytes(), b'content')
        finally:
            os.close(fd)


if __name__ == '__main__': unittest.main()
