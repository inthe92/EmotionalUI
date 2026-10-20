#!/usr/bin/env python3
"""Stage a private, no-game, supervised RG353V environmental test. Never deploy."""
import argparse,hashlib,json,shutil,subprocess,tarfile
from pathlib import Path
from arm64.verify_elf import audit
ROOT=Path(__file__).resolve().parents[1]
MOMENTS={'madrugada':'03:00','manana-temprana':'07:30','media-manana':'10:30','primera-tarde':'14:00','media-tarde':'17:30','anochecer':'20:00','noche':'22:30'}
SEASONS={'spring':'2026-03-15','summer':'2026-06-15','autumn':'2026-09-15','winter':'2026-12-15'}
def scenarios():
    result={}
    for season,date in SEASONS.items():
        for moment,hour in MOMENTS.items():result[season+'-'+moment]=dict(at=date+'T'+hour,weather='unknown',season=season)
    result.update({
      'winter-snow-christmas':dict(at='2026-12-25T16:00',weather='snow',season='winter'),
      'autumn-rain-halloween':dict(at='2026-10-31T20:00',weather='rain',season='autumn'),
      'summer-clear':dict(at='2026-06-21T10:30',weather='clear',season='summer'),
      'winter-fog':dict(at='2026-01-15T03:00',weather='fog',season='winter'),
      'cloudy':dict(at='2026-10-20T14:00',weather='cloud',season='autumn'),
      'new-year':dict(at='2027-01-01T22:30',weather='unknown',season='winter'),
      'unknown-clock':dict(weather='unknown',unknown_time=True),
      'live':dict(weather='unknown'),
    })
    return result
README='''# EmotionalUI · prueba PRIVADA RG353V / 640×480

No es instalador ni release. Incluye arte provisional de derechos pendientes embebido:
NO redistribuir. Origen: commit 80c6dcd + árbol local ambiental; consultar manifiesto.
La misma interfaz conserva fondos, fuentes, audio y materiales dentro del ELF; no se
copian ROMs/BIOS, carátulas comerciales, partidas ni configuraciones del sistema.
El audio de prueba está desactivado. No hay cliente de clima real en esta revisión.

COMPATIBILIDAD PENDIENTE: el dispositivo visto anteriormente ejecutaba dArkOS;
no se presupone equivalencia con otra versión de ArkOS. Requiere Linux aarch64,
loader /lib/ld-linux-aarch64.so.1, GLIBC >=2.36 y las bibliotecas de elf-audit.json.
Usa Slint linuxkms-software, KMS directo, escala 1, no backend simulator.
No incluye ni instala bibliotecas globales. Si el loader falla: CANCELAR y registrar;
no sustituir libc ni modificar el SO para forzar el arranque.

1. Transferir por SSH/SCP a un NUEVO directorio privado de ark cuyo nombre no tenga
   espacios, por ejemplo ~/emotional-ui-tests/environment-20261020/. No sobrescribir
   ensayos anteriores. Extraer este paquete allí y comprobar: sha256sum -c SHA256SUMS.
   Esto es copia aislada, no instalación. No modifica EmulationStation ni el arranque.
2. Con EmulationStation visible y sin juego: python3 validation_preview.py --preflight
   Solo loader/--help, modelo, DRM 640×480 y nombre del mando: NO inicia la GUI.
   Revisar /etc/os-release y systemctl cat emulationstation.service en esta consola.
   El perfil retrogame_joypad procede de una captura física anterior: NO certificado
   para la revisión actual. Sticks y R2 sin asignar. No se inventan botones.
3. Abrir una SEGUNDA conexión SSH TCP independiente; comprobar que reconecta y puede
   usar sudo -n systemctl show emulationstation.service. El propietario debe estar
   presente y autorizar expresamente interrumpir la pantalla antes del paso 4.
   Tener en esa conexión: sudo -n systemctl start emulationstation.service
   No iniciar ni detener otras unidades; no probar si hay un emulador/compositor activo.
4. Primero probar la recuperación SIN detener ES:
   python3 validation_preview.py --probe --execute --supervised --second-session-confirmed
   Inspeccionar el resultado y que ES siga visible. Esto no certifica recuperación gráfica.
5. Lista: python3 validation_preview.py --list
   Sin --execute solo imprime el plan. Ejemplo de prueba autorizada de dos minutos:
   python3 validation_preview.py --case winter-snow-christmas --view emotional --seconds 120 --execute --supervised --second-session-confirmed
   Puede añadirse --library-root /RUTA/VERIFICADA para reutilizar gamelist.xml e imágenes
   locales en SOLO LECTURA. Sin ruta usa biblioteca vacía propia. No se adivinan rutas.
   La base preview.sqlite/caché son propias; no importa historial ni inventa tres partidas.
   Continuar puede estar vacío sin sesiones fiables: es un estado esperado, no una avería.
6. Revisar cruceta, A/B, L/R, Start, foco del dock de cuatro destinos y cápsula. L/R
   conservan su contrato; cruceta+A elige cada destino. B cierra/restaura foco.
   Mantener el brillo actual. Solo el propietario, mediante controles del sistema ya
   verificados, compara brillo bajo/alto; no se escriben valores sysfs desde estos scripts.
   Select activa movimiento reducido. NO se configura ni lanza ningún emulador (--trial).
7. Salir con Start → Salir. Alternativa desde SSH: crear el archivo <unidad>.quit.request
   indicado en el plan de ejecución. Cada unidad conserva su límite independiente de
   180 s como máximo + margen: EXCLUSIVO a --trial, nunca aplicable a juegos.
   ExecStopPost pide iniciar ES aunque se pierda la conexión llamante. No hay servicios
   permanentes, watchdog de juegos, cambios de reloj, brillo, audio o arranque.
8. Confirmar VISUALMENTE ES, además de ActiveState=active. Si no se ve, leer brillo/DRM;
   no escribir ni matar procesos desconocidos. Desde la segunda sesión solicitar el
   cierre de NUESTRA vista previa y systemctl start de la unidad ES verificada. Si la
   recuperación independiente no puede verificarse: NO continuar con pruebas gráficas.

DEMO: 28 perfiles = siete claves artísticas × cuatro estaciones meteorológicas NORTE,
con fechas civiles de prueba, sin cambiar el reloj del SO. Clima desconocido en la matriz;
seis combinaciones adicionales prueban lluvia, niebla, nieve, despejado, nublado y eventos.
También live (sin fecha simulada) y unknown-clock. Live no supone hemisferio/ubicación:
la estación queda desconocida si no se configura en nuestros archivos propios. Nunca
se inventa meteorología; todos los climas elegidos explícitamente se etiquetan DEMO.
No se añade fundido entre pinturas. Las 28 claves se eligen a horas exactas; la mezcla
ambiental heredada sigue presente entre claves y queda pendiente de decisión posterior.

MEDICIÓN: unidad/PID y tiempos de humo propios, <unidad>.stdout/.stderr/-result.json.
Para RAM/CPU: obtener MainPID de la unidad y ejecutar en otra conexión
python3 sample_process.py PID --seconds 30 > medida-privada.jsonl
Registrar RSS/pico/ticks y fase (reposo/navegación/carga). No son FPS ni latencia de LCD.
Arranque visible y legibilidad los confirma la persona; ui_visible en log no es esa prueba.
La prueba prolongada queda pendiente de sesiones supervisadas sucesivas; no retirar
el límite de seguridad para simular una validación que no existe.

Licencias y procedencia están en los archivos incluidos. Checksums comprueban
integridad, no identidad pública. No publicar paquete ni logs privados.
'''
def build(binary,destination,sysroot,input_profile):
    if destination.exists():raise ValueError('Refusing to overwrite package')
    report=audit(binary,sysroot)
    if input_profile.is_symlink() or not input_profile.is_file() or input_profile.stat().st_size>65536:raise ValueError('Explicit bounded controller profile required')
    destination.mkdir(parents=True)
    def copy(source,name):
        if source.is_symlink():raise ValueError('Unexpected payload symlink')
        shutil.copy2(source,destination/name)
    copy(binary,'emotional-ui');(destination/'emotional-ui').chmod(0o755)
    for name in ['phase12_preview.py','validation_preview.py','preflight.py','sample_process.py']:
        copy(ROOT/'platform/darkos'/name,name)
    copy(input_profile,'input.toml')
    for source,name in [('LICENSE','LICENSE'),('THIRD_PARTY_LICENSES.md','THIRD_PARTY_LICENSES.md'),('AUDIO_ASSET_PROVENANCE.md','AUDIO_ASSET_PROVENANCE.md'),('assets/fonts/DejaVu-COPYRIGHT.txt','DejaVu-COPYRIGHT.txt'),('assets/scenes/daycycle/selected-room-v1/PROVENANCE.md','ROOM_PROVENANCE.md')]:copy(ROOT/source,name)
    (destination/'empty').mkdir()
    rows=scenarios()
    for name,row in rows.items():
        filename='demo-'+name+'.toml';row['config']=filename
        text='weather="'+row['weather']+'"\n'
        if 'at' in row:text+='at="'+row['at']+'"\n'
        text+='[ambient]\ntimezone="Etc/UTC"\n' if name!='live' else '[ambient]\n'
        if name not in ('live','unknown-clock'):text+='hemisphere="north"\n'
        (destination/filename).write_text(text)
    (destination/'scenarios.json').write_text(json.dumps(rows,indent=2)+'\n')
    sanitized={key:report[key] for key in ['format','machine','size_bytes','sha256','needed','glibc_versions','interpreter','rpaths','device_compatibility']}
    (destination/'elf-audit.json').write_text(json.dumps(sanitized,indent=2)+'\n')
    (destination/'README.md').write_text(README)
    files={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(destination.iterdir()) if p.is_file()}
    base=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    manifest=dict(kind='phase12-private-preview',private=True,public_redistribution=False,source_base_commit=base,source_worktree_dirty=bool(subprocess.check_output(['git','status','--porcelain'],cwd=ROOT)),backend='linuxkms-software',build_feature='linuxkms-direct',resolution=[640,480],physical_verified=False,files=files)
    (destination/'phase12-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    hashes=files|{'phase12-manifest.json':hashlib.sha256((destination/'phase12-manifest.json').read_bytes()).hexdigest()}
    (destination/'SHA256SUMS').write_text(''.join(f'{digest}  {name}\n' for name,digest in sorted(hashes.items())))
    archive=destination.with_suffix('.tar.gz')
    if archive.exists():raise ValueError('Refusing to overwrite archive')
    with tarfile.open(archive,'w:gz') as tar:tar.add(destination,arcname=destination.name)
    (archive.parent/(archive.name+'.sha256')).write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+archive.name+'\n')
    return manifest
if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--sysroot',type=Path,required=True);p.add_argument('--input-profile',type=Path,required=True);a=p.parse_args();build(a.binary,a.output,a.sysroot,a.input_profile);print(a.output)
