# Scripts anteriores

Español | [English](./README.md) | [Aplicación de escritorio](../README-ES.md)

Estos scripts antiguos de Windows son independientes de la aplicación de escritorio. Modifican ajustes de Windows para juegos, lanzadores y otros procesos; **no** cambian los ajustes visuales de los juegos. Los resultados dependen del equipo y no se garantizan mejoras de FPS, latencia o compatibilidad. Algunos mensajes que muestran los scripts hacen afirmaciones más contundentes que no deben tomarse como garantías.

> **Advertencia:** Los scripts modifican ajustes del sistema y pueden reducir la protección antivirus. Sus opciones de reversión no restauran todos los valores anteriores y pueden eliminar exclusiones de Defender que ya existían. Revisa y respalda tus ajustes antes de utilizarlos.

## Qué hacen

- Ambos scripts ofrecen opciones para lanzadores, juegos y procesos secundarios. Solicitan una preferencia de GPU a Windows y modifican el Modo Juego y el plan de energía. La opción de juego también desactiva la grabación Game DVR y agrega exclusiones de Defender para el ejecutable y toda su carpeta.
- El script en español también cambia la prioridad de CPU para futuras ejecuciones del lanzador o juego con el mismo nombre de archivo, incluso en otras carpetas. El script en inglés no cambia la prioridad de CPU.
- El plan de energía afecta a todo el equipo. Windows o la aplicación pueden ignorar la preferencia de GPU solicitada. Las exclusiones de Defender reducen la protección para la carpeta y el proceso afectados.
- Revertir elimina la preferencia de GPU y aplica valores predefinidos a otros ajustes; no restaura todos los valores anteriores. El script en español respalda la prioridad de CPU por separado e intenta restaurarla; sin respaldo, conserva el valor actual.

## Uso

1. En Windows, ejecuta `game-optimizer-es.bat` o `game-optimizer-en.bat`. Mantén `cpu-priority.ps1` junto al script en español.
2. Selecciona una opción del 1 al 3 para configurar o del 4 al 6 para revertir y escribe la ruta completa del ejecutable.
3. La configuración y reversión del perfil de juego solicitan permiso de administrador. El script en español también requiere permisos de administrador para configurar o revertir la prioridad de CPU de lanzadores y juegos. Revisa los avisos y verifica los ajustes resultantes.

Los scripts usan PowerShell y guardan logs en `legacy-scripts\logs`. Sus mensajes y opciones de reversión no equivalen a la restauración mediante transacciones de la aplicación de escritorio.

## Implementación

Las preferencias de GPU se escriben en `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` y `HKCU\System\GameConfigStore\Children`. Los scripts también modifican `AutoGameModeEnabled`, `GameDVR_Enabled`, planes de energía y exclusiones de Defender. El asistente de CPU en español utiliza Image File Execution Options y guarda su respaldo en el registro del equipo. Los cambios de CPU y las exclusiones de Defender requieren permisos de administrador. Consulta `cpu-priority.ps1` y `tests\cpu-priority.tests.ps1` para la implementación y las pruebas de CPU.

## Licencia

Consulta la [licencia](../LICENSE) del repositorio.
