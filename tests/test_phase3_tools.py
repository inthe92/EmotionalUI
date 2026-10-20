import hashlib,importlib.util,io,struct,sys,tarfile,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
from types import SimpleNamespace
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'platform/darkos'))

def module(name,path):
    spec=importlib.util.spec_from_file_location(name,ROOT/path);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
sdk=module('sdk','tools/arm64/prepare_sysroot.py')
elf=module('elf','tools/arm64/verify_elf.py')
ssh=module('collector','tools/arm64/collect_ssh.py')
smoke=module('manual_smoke','platform/darkos/manual_smoke.py')

class BuildTools(unittest.TestCase):
    def test_elf_architecture(self):
        data=bytearray(64);data[:6]=b'\x7fELF\x02\x01';struct.pack_into('<H',data,16,3);struct.pack_into('<H',data,18,183)
        self.assertEqual(elf.header(data)['machine'],'AArch64')
        struct.pack_into('<H',data,18,62)
        with self.assertRaises(ValueError):elf.header(data)
    def test_not_a_macos_executable(self):
        with self.assertRaises(ValueError):elf.header(b'\xcf\xfa\xed\xfe'+bytes(64))
    def test_unsafe_archive_names(self):
        for path in ['/etc/passwd','../escape','usr/../../escape']:
            with self.assertRaises(ValueError):sdk.safe_member_path(path)
    def test_root_tar_entry_is_allowed(self):
        payload=io.BytesIO()
        with tarfile.open(fileobj=payload,mode='w:xz') as tar:
            directory=tarfile.TarInfo('.');directory.type=tarfile.DIRTYPE;tar.addfile(directory)
            item=tarfile.TarInfo('usr/include/example.h');item.size=4;tar.addfile(item,io.BytesIO(b'test'))
        with tempfile.TemporaryDirectory() as temp,patch.object(sdk.subprocess,'check_output',side_effect=[b'data.tar.xz\n',payload.getvalue()]):
            root=Path(temp).resolve();sdk.unpack(Path('fixture.deb'),root);self.assertEqual((root/'usr/include/example.h').read_bytes(),b'test')
    def test_symlink_escape_is_rejected(self):
        payload=io.BytesIO()
        with tarfile.open(fileobj=payload,mode='w:xz') as tar:
            link=tarfile.TarInfo('usr/evil');link.type=tarfile.SYMTYPE;link.linkname='../../../outside';tar.addfile(link)
        with tempfile.TemporaryDirectory() as temp,patch.object(sdk.subprocess,'check_output',side_effect=[b'data.tar.xz\n',payload.getvalue()]):
            with self.assertRaises(ValueError):sdk.unpack(Path('fixture.deb'),Path(temp).resolve())
    def test_ssh_destination_validation(self):
        for host in ['ark@console.local','my-console','ark@192.0.2.4']:self.assertTrue(ssh.valid_host(host))
        for host in ['-oProxyCommand=anything','host;exit','host with spaces','$(command)']:self.assertFalse(ssh.valid_host(host))
    def test_active_frontend_prevents_launch(self):
        data=bytearray(64);data[:6]=b'\x7fELF\x02\x01';struct.pack_into('<H',data,18,183)
        with tempfile.TemporaryDirectory() as temp:
            binary=Path(temp)/'app';binary.write_bytes(data);logs=Path(temp)/'logs'
            args=['smoke',str(binary),str(logs),'--sha256',hashlib.sha256(data).hexdigest(),'--recovery-confirmed']
            with patch.object(sys,'argv',args),patch.object(smoke.platform,'system',return_value='Linux'),patch.object(smoke.platform,'machine',return_value='aarch64'),patch.dict(smoke.os.environ,{'SSH_CONNECTION':'synthetic'}),patch.object(smoke.subprocess,'run',return_value=SimpleNamespace(stdout='emulationstatio\n')),patch.object(smoke.subprocess,'Popen') as launch:
                with self.assertRaisesRegex(SystemExit,'graphical frontend is active'):smoke.main()
                launch.assert_not_called()
            self.assertFalse(logs.exists())
if __name__=='__main__':unittest.main()
