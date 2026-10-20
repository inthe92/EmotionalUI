# Instalación conservadora · candidato privado

Objetivo de producto: descargar, copiar, probar y elegir interfaz en diez minutos. **No medido, no conseguido**: todavía faltan entrada gráfica desde dArkOS, recuperación física y selección de frontend principal. No se exige SSH al futuro usuario final; los diagnósticos actuales pertenecen a la preparación técnica.

`tools/package_trial.py --binary ELF --output CARPETA_NUEVA` prepara un candidato privado, no lo despliega. Incluye un ejecutable ARM64, scripts install/restore/uninstall/trial, manage.py, Atlas experimental opcional, config informativa, licencias y fuente correspondiente del proyecto. Arte y audio predeterminados ya están embebidos; no se duplican como otro conjunto runtime. `package.json` inventaría hashes; SHA256SUMS cubre payload y manifiesto. Es integridad, no firma ni autenticación del editor. La revisión de dependencias transitivas y derechos de la referencia artística bloquea distribución pública.

Python >=3.8 estándar en destino, sin framework nuevo. `install.sh --yes` verifica Linux ARM64, modelo RG353V, identificación explícita dArkOS en os-release y resolución por el loader glibc. Una instalación legítima con nombres distintos puede ser rechazada: hasta conocer esa revisión no se relaja la comprobación. El chequeo de loader no valida DRM, mandos ni retorno.

Solo se copian archivos propios a un directorio nuevo (por defecto ~/.local/opt/EmotionalUI). Nunca se editan servicios, frontend, ROMs o partidas. Se comprueban hashes, symlinks, espacio con margen 16 MiB, destino ajeno y manifiesto. Copia en directorio .installing y promoción por rename. Repetir el mismo paquete no duplica; una versión diferente requiere destino nuevo. No hay actualización destructiva.

La confirmación es `--yes`, con alcance explícito. No se realiza cambio delicado por defecto. `manage.py install-main` falla con explicación y sin cambios: no existe un botón de instalación principal ficticio. Copiar archivos propios no habilita la sustitución del frontend. Esta versión no ofrece aún un instalador gráfico para principiantes.

Las pruebas usan árboles temporales, binarios de texto no ejecutados y plataforma simulada. Cubren confirmación, duplicados, espacio, plataforma/dependencias no establecidas, checksums, destino ajeno, symlinks, conflictos, copia interrumpida y restauración/desinstalación. No son una instalación real en dArkOS.
