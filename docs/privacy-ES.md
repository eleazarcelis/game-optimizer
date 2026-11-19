# Política de privacidad

Español | [English](./privacy.md)

Game Optimizer no recopila, almacena de forma remota ni transmite datos personales, y no tiene telemetría, analítica ni cuentas.

## Uso de red

- La única petición de red que hace la propia app es a la API de GitHub (`api.github.com`) para consultar la última versión estable. Ocurre solo cuando pulsas la comprobación de actualizaciones en Ajustes > Acerca de, o al iniciar si activas la comprobación opcional (desactivada por defecto).
- Esa petición va directamente de tu equipo a GitHub. GitHub recibe los datos habituales de conexión (como tu dirección IP) y los trata según la [declaración de privacidad de GitHub](https://docs.github.com/es/site-policy/privacy-policies/github-general-privacy-statement). Game Optimizer no envía ningún identificador propio.
- Descargar una actualización o reportar un problema abre una página de GitHub en tu navegador. Lo que ocurre allí depende de tu navegador y de GitHub.
- La app no usa ningún otro servicio de red.

## Datos guardados en tu equipo

- Tu biblioteca, los perfiles, los ajustes, el diario de transacciones y las copias de seguridad usadas para restaurar valores anteriores se guardan localmente en la carpeta de datos de la app y nunca se suben.
- Los registros se escriben localmente en la carpeta de registros de la app y nunca se suben. Si reportas un error, compártelos solo si tú decides hacerlo.

## Qué cambia la app en tu sistema

Game Optimizer cambia los ajustes de Windows que le pides cambiar: prioridad de CPU mediante el registro, preferencia de GPU, ajustes opcionales de plan de energía y Modo juego, y exclusiones opcionales de Microsoft Defender. Pide confirmación antes, registra cada cambio en su diario y puede restaurar los valores anteriores. Las exclusiones de Defender están desactivadas por defecto, solo afectan a la carpeta y al proceso que elijas, y reducen la protección de esos elementos mientras estén activas.

## Componentes de terceros

Game Optimizer se compila con dependencias de código abierto. Conservan sus propias licencias y ninguna se usa para enviar datos a otros sistemas.

## Cambios en esta política

Los cambios de esta política se publican en este repositorio y quedan en su historial.

## Contacto

Dudas o inquietudes: abre una incidencia en [github.com/eleazarcelis/game-optimizer/issues](https://github.com/eleazarcelis/game-optimizer/issues).
