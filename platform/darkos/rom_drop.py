#!/usr/bin/env python3
"""Temporary, token-protected LAN ROM receiver. Never starts on import.

The destination inventory is read from the installed ES configuration; ROMs are
never executed, overwritten or extracted. No dependencies beyond Python 3.
"""
import argparse
import ctypes
import errno
import hmac
import ipaddress
import json
import os
import re
import secrets
import shutil
import socket
import stat
import subprocess
import sys
import time
import tomllib
import urllib.parse
import xml.etree.ElementTree as ET
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

MAX_XML = 8 * 1024 * 1024
MAX_FILE = 16 * 1024 * 1024 * 1024
BLOCKED_SYSTEMS = {'ports', 'movies', 'cmds', 'bios'}
BLOCKED_EXTENSIONS = {'.sh', '.bash', '.py', '.pl', '.rb', '.bat', '.cmd', '.ps1',
                      '.so', '.dll', '.elf', '.appimage', '.exe', '.com', '.sav', '.srm', '.state'}


def inventory(systems, home, library):
    library = Path(library).resolve(strict=True)
    home = Path(home).resolve(strict=True)
    data = Path(systems).read_bytes()
    if len(data) > MAX_XML or b'<!DOCTYPE' in data.upper() or b'<!ENTITY' in data.upper():
        raise ValueError('Inventario de sistemas no válido')
    doc = ET.fromstring(data)
    if doc.tag != 'systemList' or len(doc.findall('system')) > 4096:
        raise ValueError('Inventario de sistemas no válido')
    result, roots = [], set()
    for node in doc.findall('system'):
        name = (node.findtext('name') or '').strip()
        title = (node.findtext('fullname') or name).strip()[:80]
        raw = (node.findtext('path') or '').strip()
        if not name or name.lower() in BLOCKED_SYSTEMS or not raw:
            continue
        if raw.startswith('~/'):
            raw = str(home)+raw[1:]
        path = Path(raw)
        if not path.is_absolute() or '..' in path.parts:
            continue
        try:
            root = path.resolve(strict=True)
        except (OSError, RuntimeError):
            continue
        if (not root.is_dir() or root == library or not root.is_relative_to(library)
                or root.name.lower() in BLOCKED_SYSTEMS or root in roots):
            continue
        extensions = sorted({ext.lower() for ext in (node.findtext('extension') or '').split()
                             if re.fullmatch(r'\.[A-Za-z0-9]{1,23}', ext)
                             and ext.lower() not in BLOCKED_EXTENSIONS})
        if extensions:
            roots.add(root)
            result.append({'id': 's'+str(len(result)), 'name': title or name,
                           'extensions': extensions, 'root': root})
    return result


def filename(value):
    if (not value or value.startswith('.') or '/' in value or '\\' in value or value in ('.', '..')
            or any(ord(c) < 32 or ord(c) == 127 for c in value)
            or len(value.encode('utf-8')) > 200):
        raise ValueError('Nombre de archivo no válido')
    suffix = Path(value).suffix.lower()
    if not re.fullmatch(r'\.[a-z0-9]{1,23}', suffix) or suffix in BLOCKED_EXTENSIONS:
        raise ValueError('Tipo de archivo no admitido')
    return suffix


def publish(directory, temporary, name, stopping):
    """Publish without overwriting, including filesystems without hard links."""
    if sys.platform == 'linux':
        libc = ctypes.CDLL(None, use_errno=True)
        rename = getattr(libc, 'renameat2', None)
        if rename is not None:
            rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
            rename.restype = ctypes.c_int
            if rename(directory, os.fsencode(temporary), directory, os.fsencode(name), 1) == 0:
                return  # Linux RENAME_NOREPLACE consumed the temporary file.
            code = ctypes.get_errno()
            if code == errno.EEXIST:
                raise FileExistsError(code, 'Destination exists')
            if code not in (errno.ENOSYS, errno.EINVAL, errno.EOPNOTSUPP, errno.EPERM):
                raise OSError(code, 'Cannot publish upload')
    try:
        os.link(temporary, name, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False)
        return
    except OSError as error:
        if error.errno not in (errno.ENOSYS, errno.EINVAL, errno.EOPNOTSUPP, errno.EPERM):
            raise
    # Some removable filesystems support neither rename-noreplace nor links.
    # O_EXCL preserves all existing files; a stopped/failed copy removes only
    # the inode just created. A sudden power loss can leave an incomplete file.
    source_fd = os.open(temporary, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory)
    try:
        destination_fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=directory)
        identity = os.fstat(destination_fd)
        try:
            with os.fdopen(destination_fd, 'wb') as out, os.fdopen(source_fd, 'rb', closefd=False) as source:
                while True:
                    if stopping(): raise ValueError('Envío cancelado')
                    block = source.read(1024*1024)
                    if not block: break
                    out.write(block)
                out.flush()
                os.fsync(out.fileno())
        except BaseException:
            try:
                current = os.stat(name, dir_fd=directory, follow_symlinks=False)
                if (current.st_dev, current.st_ino) == (identity.st_dev, identity.st_ino):
                    os.unlink(name, dir_fd=directory)
            except OSError:
                pass
            raise
    finally:
        os.close(source_fd)


def restrict_to_launchers(systems, profile):
    """Only offer destinations which the current functional library can scan."""
    raw = Path(profile).read_bytes()
    if len(raw) > 512 * 1024:
        raise ValueError('Perfil demasiado grande')
    config = tomllib.loads(raw.decode('utf-8'))
    launchers = config.get('launchers', [])
    if not isinstance(launchers, list) or len(launchers) > 128:
        raise ValueError('Perfil no válido')
    allowed = {}
    for launcher in launchers:
        path = Path(launcher.get('root', ''))
        if not path.is_absolute():
            continue
        try:
            path = path.resolve(strict=True)
        except OSError:
            continue
        allowed.setdefault(path, set()).update(e.lower() for e in launcher.get('extensions', []) if isinstance(e, str))
    result = []
    for system in systems:
        extensions = sorted(set(system['extensions']) & allowed.get(system['root'], set()))
        if extensions:
            result.append(dict(system, extensions=extensions))
    return result


def local_address():
    # UDP connect asks the kernel which interface a route would use; it sends no packet.
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    def usable(address):
        ip = ipaddress.ip_address(address)
        return not (ip.is_loopback or ip.is_unspecified or ip.is_global)
    try:
        sock.connect(('192.0.2.1', 9))
        address = sock.getsockname()[0]
        if usable(address):
            return address
    except OSError:
        pass
    finally:
        sock.close()
    # A private Wi-Fi LAN can work without an Internet/default route.
    ip_command = next((str(p) for p in (Path('/usr/sbin/ip'), Path('/sbin/ip'), Path('/usr/bin/ip')) if p.is_file()), None)
    if ip_command:
        try:
            result = subprocess.run([ip_command, '-j', '-4', 'addr', 'show', 'up', 'scope', 'global'],
                                    capture_output=True, text=True, timeout=3, check=True)
            interfaces = json.loads(result.stdout)
            interfaces.sort(key=lambda i: not str(i.get('ifname', '')).startswith(('wlan', 'wl')))
            for interface in interfaces:
                for item in interface.get('addr_info', []):
                    address = item.get('local', '')
                    if usable(address): return address
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    raise ValueError('Conecta la consola a una red local')


PAGE = r'''<!doctype html><html lang="es"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Emotional Drop</title><style>
:root{color-scheme:dark}body{font:16px system-ui,sans-serif;background:#101c2d;color:#f8f7f4;margin:0;padding:24px}main{max-width:600px;margin:auto}h1{margin-bottom:4px}p{line-height:1.45;color:#cbd6e4}label,.item{display:block;background:#26394d;border:1px solid #516779;border-radius:14px;padding:16px;margin:12px 0}input,select,button{font:inherit}input[type=file]{max-width:100%}select{display:block;width:100%;margin-top:10px;padding:9px;border-radius:8px}button{background:#f8d7a4;color:#30271e;border:0;border-radius:22px;padding:12px 24px;font-weight:700}button:disabled{opacity:.45}progress{width:100%;margin-top:12px}.error{color:#ffb1a8}.ok{color:#a8e8c5}</style></head><body><main>
<h1>Emotional Drop</h1><p>Elige uno o varios archivos. Si una extensión corresponde a varias consolas, selecciona el destino de cada juego.</p><label>Archivos de juegos<input id="files" type="file" multiple></label><div id="list"></div><button id="send" disabled>Enviar a la consola</button><p id="notice" role="status">Conectando…</p><p>El envío es local. Los archivos existentes no se reemplazan.</p></main><script>
'use strict';const token=location.hash.slice(1);history.replaceState(null,'',location.pathname);const files=document.querySelector('#files'),list=document.querySelector('#list'),notice=document.querySelector('#notice'),send=document.querySelector('#send');let systems=[],rows=[];
function message(t,style){notice.textContent=t;notice.className=style||''}function options(file){const pos=file.name.lastIndexOf('.');const ext=pos>=0?file.name.slice(pos).toLowerCase():'';return systems.filter(s=>s.extensions.includes(ext))}
function render(){list.replaceChildren();rows=Array.from(files.files).map(file=>{const row=document.createElement('div');row.className='item';const name=document.createElement('strong');name.textContent=file.name;row.append(name);const select=document.createElement('select');const choices=options(file);if(choices.length!==1){const placeholder=document.createElement('option');placeholder.value='';placeholder.textContent=choices.length?'Elige la consola':'Extensión no reconocida';select.append(placeholder)}for(const system of choices){const option=document.createElement('option');option.value=system.id;option.textContent=system.name;select.append(option)}row.append(select);const status=document.createElement('p');status.textContent=choices.length===1?'Destino detectado':'Selecciona un destino';row.append(status);list.append(row);select.onchange=()=>{status.textContent=select.value?'Destino elegido':'Selecciona un destino';validate()};return{file,select,status}});validate()}
function validate(){send.disabled=!rows.length||rows.every(r=>r.status.className==='ok')||rows.some(r=>r.status.className!=='ok'&&!r.select.value)}
files.onchange=render;send.onclick=async()=>{send.disabled=true;files.disabled=true;rows.forEach(r=>r.select.disabled=true);for(const row of rows){if(row.status.className==='ok')continue;row.status.textContent='Enviando…';try{const xhr=new XMLHttpRequest();await new Promise((resolve,reject)=>{xhr.open('POST','/api/file?system='+encodeURIComponent(row.select.value)+'&name='+encodeURIComponent(row.file.name));xhr.setRequestHeader('X-Emotional-Token',token);xhr.upload.onprogress=e=>{if(e.lengthComputable)row.status.textContent='Enviando '+Math.round(e.loaded/e.total*100)+' %'};xhr.onerror=()=>reject(Error('Conexión interrumpida'));xhr.onload=()=>{let result;try{result=JSON.parse(xhr.responseText)}catch(_){return reject(Error('Respuesta no válida'))}if(xhr.status!==201)return reject(Error(result.error||'No se ha podido guardar'));resolve()};xhr.send(row.file)});row.status.textContent='Guardado';row.status.className='ok'}catch(e){row.status.textContent=e.message;row.status.className='error'}}files.disabled=false;rows.forEach(r=>r.select.disabled=r.status.className==='ok');message('Envío terminado. Revisa el resultado de cada archivo.');validate()};
fetch('/api/systems',{headers:{'X-Emotional-Token':token},cache:'no-store'}).then(r=>{if(!r.ok)throw Error('Enlace no válido o caducado');return r.json()}).then(data=>{systems=data.systems;message('Conectado. Selecciona tus archivos.');render()}).catch(e=>message(e.message,'error'));
</script></body></html>'''.encode('utf-8')


def parent_alive(pid, start):
    try:
        raw = Path('/proc')/str(pid)/'stat'
        tail = raw.read_text().rsplit(')', 1)[1].split()
        return len(tail) > 19 and tail[0] != 'Z' and tail[19] == start
    except (OSError, IndexError):
        return False


class DropServer(HTTPServer):
    allow_reuse_address = False
    def __init__(self, address, systems, token, stop_path=None, parent=None):
        super().__init__(address, Handler)
        self.systems = systems
        self.token = token
        self.last_activity = time.monotonic()
        self.stop_path = stop_path
        self.parent = parent
        self.timeout = 1

    def stopping(self):
        return ((self.stop_path is not None and self.stop_path.exists())
                or (self.parent is not None and not parent_alive(*self.parent)))


class Handler(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def setup(self):
        super().setup()
        self.connection.settimeout(5)
    def log_message(self, *_args):
        pass  # Do not log tokens, file names or local network addresses.
    def response(self, status, body, kind='application/json; charset=utf-8'):
        self.send_response(status)
        self.send_header('Content-Type', kind)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.send_header('Content-Security-Policy', "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'")
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(body)
        self.close_connection = True
    def error(self, status, message):
        self.response(status, json.dumps({'error': message}, ensure_ascii=False).encode())
    def authorized(self):
        origin = self.headers.get('Origin')
        expected = 'http://'+self.headers.get('Host', '')
        return ((origin is None or origin == expected)
                and hmac.compare_digest(self.headers.get('X-Emotional-Token', ''), self.server.token))
    def do_GET(self):
        self.server.last_activity = time.monotonic()
        if self.path == '/':
            self.response(200, PAGE, 'text/html; charset=utf-8')
        elif self.path == '/api/systems' and self.authorized():
            data = [{'id': s['id'], 'name': s['name'], 'extensions': s['extensions']} for s in self.server.systems]
            self.response(200, json.dumps({'systems': data}, ensure_ascii=False).encode())
        else:
            self.error(403, 'Enlace no válido o caducado')
    def do_POST(self):
        if not self.authorized():
            return self.error(403, 'Enlace no válido o caducado')
        parsed = urllib.parse.urlsplit(self.path)
        try:
            fields = urllib.parse.parse_qs(parsed.query, keep_blank_values=True, max_num_fields=2)
        except ValueError:
            return self.error(400, 'Solicitud no válida')
        if parsed.path != '/api/file' or set(fields) != {'system', 'name'} or any(len(v) != 1 for v in fields.values()):
            return self.error(400, 'Solicitud no válida')
        try:
            name = fields['name'][0]
            suffix = filename(name)
            system = next(s for s in self.server.systems if s['id'] == fields['system'][0])
            if suffix not in system['extensions']:
                raise ValueError('La extensión no corresponde a esa consola')
            if self.headers.get('Transfer-Encoding') is not None:
                raise ValueError('Transferencia no compatible')
            if len(self.headers.get_all('Content-Length', [])) != 1:
                raise ValueError('Tamaño no válido')
            length = int(self.headers.get('Content-Length', ''))
            if length <= 0 or length > MAX_FILE:
                raise ValueError('Tamaño de archivo no válido')
            root = system['root']
            if shutil.disk_usage(root).free < length + 4 * 1024 * 1024:
                raise ValueError('No queda espacio suficiente')
            directory = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
            temporary = '.emotional-drop-'+secrets.token_hex(16)+'.part'
            try:
                fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=directory)
                try:
                    with os.fdopen(fd, 'wb') as out:
                        remaining = length
                        while remaining:
                            if self.server.stopping():
                                raise ValueError('Envío cancelado')
                            block = self.rfile.read(min(1024*1024, remaining))
                            if not block:
                                raise ValueError('Envío interrumpido')
                            out.write(block)
                            remaining -= len(block)
                            self.server.last_activity = time.monotonic()
                        out.flush()
                        os.fsync(out.fileno())
                    if self.server.stopping():
                        raise ValueError('Envío cancelado')
                    publish(directory, temporary, name, self.server.stopping)
                    try:
                        os.fsync(directory)
                    except OSError as error:
                        if error.errno not in (errno.EINVAL, errno.EOPNOTSUPP):
                            raise
                finally:
                    try:
                        os.unlink(temporary, dir_fd=directory)
                    except FileNotFoundError:
                        pass  # renameat2 already consumed it
            finally:
                os.close(directory)
        except FileExistsError:
            return self.error(409, 'Ya existe un archivo con ese nombre; no se ha reemplazado')
        except (StopIteration, ValueError, UnicodeError):
            return self.error(400, 'Archivo o destino no válido')
        except (OSError, OverflowError):
            return self.error(500, 'No se ha podido guardar; comprueba la conexión y el espacio')
        self.server.last_activity = time.monotonic()
        self.response(201, b'{"ok":true}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--status', type=Path, required=True)
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--parent-pid', type=int, required=True)
    parser.add_argument('--parent-start', required=True)
    args = parser.parse_args()
    from player_menu import installed_systems
    systems_path, library, _ = installed_systems()
    systems = restrict_to_launchers(inventory(systems_path, Path.home(), library), args.profile)
    if not systems:
        raise ValueError('No se han encontrado sistemas compatibles')
    address = local_address()
    token = secrets.token_urlsafe(24)
    stop_path = args.status.with_suffix('.stop')
    parent = (args.parent_pid, args.parent_start)
    if args.parent_pid <= 1 or not args.parent_start.isdigit() or not parent_alive(*parent):
        raise ValueError('La sesión ya no está activa')
    server = DropServer((address, 0), systems, token, stop_path, parent)
    url = 'http://'+address+':'+str(server.server_port)+'/#'+token
    status = args.status
    if (not status.is_absolute() or not re.fullmatch(r'emotional-launch-\d+-\d+', status.parent.name)
            or not re.fullmatch(r'drop-\d+\.json', status.name)
            or status.parent.is_symlink() or stat.S_IMODE(status.parent.stat().st_mode) != 0o700
            or status.parent.stat().st_uid != os.getuid()):
        raise ValueError('Directorio de sesión no válido')
    fd = os.open(status, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as out:
        json.dump({'url': url}, out)
        out.flush()
        os.fsync(out.fileno())
    try:
        started = time.monotonic()
        while (time.monotonic()-server.last_activity < 20*60
               and time.monotonic()-started < 60*60 and not server.stopping()):
            server.handle_request()
    finally:
        server.server_close()
        status.unlink(missing_ok=True)
        stop_path.unlink(missing_ok=True)
        # The supervisor removes its owner marker on exit. If the receiver
        # finished later, remove the now-empty private IPC directory as well.
        try:
            status.parent.rmdir()
        except OSError:
            pass


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, ET.ParseError):
        raise SystemExit('No se pudo iniciar Emotional Drop')
