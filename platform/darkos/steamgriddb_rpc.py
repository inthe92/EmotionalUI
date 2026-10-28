#!/usr/bin/env python3
"""Bounded stdlib RPC; no ROM/catalog writes and no token in arguments/output."""
import importlib.util
import json
from pathlib import Path
import sys

local=Path(__file__).resolve().parent/'steamgriddb_client.py'
source=local if local.is_file() else Path(__file__).resolve().parents[2]/'tools/steamgriddb_artwork.py'
spec=importlib.util.spec_from_file_location('steamgriddb_client',source)
provider=importlib.util.module_from_spec(spec);spec.loader.exec_module(provider)


def handle(request,client=None):
    operation=request.get('operation')
    client=client or provider.Client(provider.load_key())
    if operation=='search':
        title=request.get('query','').strip()
        if not title or len(title)>120:raise ValueError('Escribe un nombre de hasta 120 caracteres.')
        games=client.search(title)
        return {'games':[{k:game.get(k) for k in ('id','name','release_date')} for game in games[:20]]}
    if operation=='grids':
        grids=client.grids(provider.positive_id(request.get('game_id')))
        return {'grids':[{k:g.get(k) for k in ('id','url','author','style','language')} for g in grids if provider.pick_grid([g]) is not None][:50]}
    if operation=='preview':
        url=provider.checked_cdn(request.get('url'))
        raw=client.image(url)
        if raw.startswith(b'\x89PNG\r\n\x1a\n'):
            import struct
            offset=8
            while offset+12<=len(raw):
                size=struct.unpack('>I',raw[offset:offset+4])[0]
                if size>provider.MAX_IMAGE or offset+size+12>len(raw):raise ValueError('PNG incompleto.')
                kind=raw[offset+4:offset+8]
                if kind==b'acTL':raise ValueError('Solo se admiten imágenes estáticas.')
                offset+=12+size
                if kind==b'IEND':break
            extension='.png'
        elif raw.startswith(b'\xff\xd8\xff'):extension='.jpg'
        else:raise ValueError('La imagen no es PNG/JPEG estático.')
        root=Path.home()/'.local/share/emotional-ui-player/steamgriddb'
        root.mkdir(parents=True,exist_ok=True,mode=0o700)
        if root.is_symlink():raise ValueError('La carpeta de carátulas debe ser privada.')
        path=root/('raw-'+provider.digest(raw)+extension)
        if path.exists():
            if path.is_symlink() or path.read_bytes()!=raw:raise ValueError('La caché no coincide; se conserva sin reemplazar.')
        else:provider.atomic_write(path,raw)
        provider.save_json(path.with_suffix('.source.json'),{'provider':'steamgriddb','url':url,'sha256':provider.digest(raw),'grid_id':request.get('grid_id'),'author':request.get('author')})
        return {'path':str(path),'root':str(root)}
    raise ValueError('Acción de SteamGridDB desconocida.')


def main():
    data=sys.stdin.buffer.read(16385)
    if len(data)>16384:raise ValueError('Solicitud demasiado grande.')
    print(json.dumps(handle(json.loads(data))))


if __name__=='__main__':
    try:main()
    except (ValueError,KeyError,TypeError,OSError) as error:
        message=str(error)
        if 'Configure' in message:message='Falta configurar la clave privada de SteamGridDB en este equipo.'
        print(json.dumps({'error':message}));sys.exit(1)
