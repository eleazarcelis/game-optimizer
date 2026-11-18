# Detalles de recuperación e implementación

Español | [English](./recovery.md)

## Detalles de implementación y recuperación

La app está construida con Tauri 2, React y TypeScript. La biblioteca, los perfiles y los diarios de transacciones se guardan en la carpeta de la aplicación dentro de `%LOCALAPPDATA%`; los logs usan el directorio de registros de Tauri para Windows. Reemplazar el ejecutable portátil en la misma cuenta de Windows no elimina esos archivos.

Valores predefinidos elimina permanentemente los ajustes de CPU/GPU habilitados, incluidos los de una optimización activa, y descarta el respaldo correspondiente. Si se interrumpe, se conserva información para su recuperación. Las transacciones antiguas de valores predefinidos siguen pudiéndose restaurar o limpiar explícitamente.

La restauración de Defender conserva las exclusiones preexistentes y verifica los cambios con elevación. Las exclusiones de carpetas superpuestas y las exclusiones de proceso para ejecutables con la misma identidad física se coordinan independientemente, incluso entre usuarios. Los diarios antiguos sin identidad del recurso requieren inspección manual. No muevas ni redirijas las carpetas o enlaces del ejecutable mientras se aplica o restaura una exclusión. Las reservas del equipo se guardan en `%PROGRAMDATA%\GameOptimizer\resources` y las nuevas reservas de Defender en `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; no las elimines mientras haya cambios activos. Restaura las transacciones antiguas de Defender antes de crear nuevas exclusiones. Estas reservas coordinan la aplicación, pero Windows u otros programas pueden modificar Defender por separado. Restaura las exclusiones independientes con esta versión o una posterior, no con una compilación anterior.

Si una restauración omite exclusiones de Defender modificadas externamente, la app libera su reserva sin cambiar esas exclusiones. Antes de aplicar otro perfil con Defender, también revisa diarios completados de versiones anteriores y libera reservas pendientes únicamente si ambas exclusiones están ausentes y las identidades de los recursos siguen coincidiendo.

Las acciones globales modifican `AutoGameModeEnabled` y `GameDVR_Enabled` del usuario actual de Windows; su efectividad depende de Windows y de las directivas existentes. El diario guarda los valores y tipos anteriores antes de escribir. Tras una interrupción, usa Restaurar en ajustes globales; los cambios externos no se sobrescriben.

## Pendiente

Las instantáneas de Defender registran el volumen y el identificador de archivo de Windows. La restauración de CPU recupera el valor de prioridad, pero puede dejar claves IFEO vacías creadas al aplicar el ajuste. También faltan pruebas exhaustivas de interfaz e interrupciones.
