"""Console RPC uses stdlib only; requests are explicit and artifacts stay private."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from PIL import Image

ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('sgdb_rpc',ROOT/'platform/darkos/steamgriddb_rpc.py')
rpc=importlib.util.module_from_spec(spec);spec.loader.exec_module(rpc)

class Client:
    def __init__(self):self.queries=[];self.game_ids=[]
    def search(self,title):self.queries.append(title);return [{'id':12,'name':'Same title','release_date':None}]
    def grids(self,id):self.game_ids.append(id);return [{'id':19,'width':600,'height':900,'url':'https://cdn2.steamgriddb.com/grid/a.png','author':{'name':'Artist'},'style':'alternate'}]
    def image(self,url):
        stream=io.BytesIO();Image.new('RGB',(600,900),'blue').save(stream,format='PNG');return stream.getvalue()

class Rpc(unittest.TestCase):
    def test_search_and_grids_return_choices_without_binding_a_rom_or_choosing_first(self):
        client=Client();result=rpc.handle({'operation':'search','query':'Same title'},client)
        self.assertEqual(result,{'games':[{'id':12,'name':'Same title','release_date':None}]})
        self.assertEqual(client.game_ids,[])
        result=rpc.handle({'operation':'grids','game_id':12},client)
        self.assertEqual(client.game_ids,[12]);self.assertEqual(result['grids'][0]['id'],19)
    def test_preview_has_bounded_provider_path_and_provenance_without_key(self):
        with tempfile.TemporaryDirectory() as folder,patch.object(Path,'home',return_value=Path(folder)):
            result=rpc.handle({'operation':'preview','url':'https://cdn2.steamgriddb.com/grid/a.png','grid_id':19,'author':'Artist'},Client())
            path=Path(result['path']);self.assertTrue(path.is_relative_to(Path(folder)))
            self.assertEqual(path.stat().st_mode&0o777,0o600)
            source=json.loads(path.with_suffix('.source.json').read_text())
            self.assertEqual(source['grid_id'],19);self.assertEqual(source['author'],'Artist')
            self.assertNotIn('key',source)
    def test_preview_rejects_off_provider_download_and_animated_png(self):
        with self.assertRaises(ValueError):rpc.handle({'operation':'preview','url':'http://127.0.0.1/private'},Client())
        stream=io.BytesIO();a=Image.new('RGB',(600,900),'red')
        a.save(stream,format='PNG',save_all=True,append_images=[Image.new('RGB',(600,900),'blue')],duration=50)
        client=Client();client.image=lambda url:stream.getvalue()
        with self.assertRaisesRegex(ValueError,'estáticas'):
            rpc.handle({'operation':'preview','url':'https://cdn2.steamgriddb.com/grid/a.png'},client)

if __name__=='__main__':unittest.main()
