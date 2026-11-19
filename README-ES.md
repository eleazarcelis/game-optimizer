# Game Optimizer

Español | [English](./README.md)

Game Optimizer es una aplicación de escritorio para Windows que organiza juegos y ayuda a gestionar cómo se ejecutan sus procesos en Windows. Modifica ajustes seleccionados del sistema; **no cambia la configuración gráfica del juego**, como la resolución, la calidad visual o los efectos. Los resultados dependen del juego, el equipo y Windows; no se garantiza una mejora de rendimiento o FPS.

**Enfoque en la estabilidad más que en los FPS.** En las pruebas del autor con juegos reales, el beneficio principal ha sido una entrega de fotogramas más consistente, con menos stuttering, más que un mayor promedio de FPS. Los ajustes que gestiona buscan reducir la interferencia de otros procesos y del sistema. El efecto varía según el juego, el equipo y Windows, y no está garantizado.

> **Trabajo en curso:** la aplicación todavía no reemplaza todas las funciones de los scripts anteriores.

## Funcionalidades actuales

- **Biblioteca:** Agrega ejecutables de Windows con el selector y asigna un perfil a cada uno. Desde su tarjeta puedes iniciar un juego o abrir su carpeta. Iniciarlo no aplica el perfil automáticamente.
- **Detección de Steam:** El botón de búsqueda de la biblioteca detecta Steam y sus bibliotecas instaladas; en Configuración > Detección automática también puedes buscar Steam o sus juegos por separado. Revisa los resultados y selecciona el ejecutable correcto antes de agregar: las carpetas pueden contener instaladores o herramientas. Steam recibe el perfil Lanzador y los juegos el perfil Juego; no se duplican ni optimizan entradas existentes. Las carpetas enlazadas y los límites de búsqueda pueden hacer que los resultados estén incompletos; la app lo advierte.
- **Organización:** Elige una grilla o lista, marca favoritos, ordena por nombre o acomoda los elementos manualmente. Puedes agrupar nombres relacionados, aunque la agrupación automática puede equivocarse. El orden manual también admite el teclado; los favoritos permanecen primero.
- **Perfiles:** En Configuración > Perfiles, ajusta los perfiles incluidos o crea los tuyos. Un perfil puede solicitar una prioridad de CPU y una preferencia de GPU para un ejecutable. La prioridad de CPU afecta a futuras ejecuciones con el mismo nombre, incluso en otras carpetas, y requiere permiso de administrador. La preferencia de GPU solicita a Windows qué GPU usar; Windows o la aplicación pueden elegir otra.
- **Prioridad del árbol de procesos:** Al iniciar desde la biblioteca un ejecutable con prioridad de CPU aplicada y verificada, Windows también aplica esa prioridad mediante un Job Object a los procesos hijos compatibles, sin un servicio de monitoreo. Los procesos ya abiertos, los creados fuera del árbol o mediante otros servicios pueden quedar excluidos; otros límites de Windows también pueden prevalecer. Si no se puede configurar, la aplicación se inicia y muestra una advertencia. La prioridad del Job Object permanece hasta que termina el árbol iniciado, incluso si se restaura el perfil o se cierra Game Optimizer; restaura o cambia el perfil antes de iniciar una nueva ejecución. Los lanzamientos externos siguen usando únicamente el ajuste por nombre de ejecutable.
- **Aplicaciones de Microsoft Store:** Los ejecutables de paquetes registrados en WindowsApps se activan mediante Windows. Si hay una prioridad de CPU aplicada, se advierte que no se puede asignar el árbol al Job Object; el ajuste por nombre de ejecutable sigue vigente. En paquetes con varias aplicaciones, una debe coincidir con el nombre del ejecutable.
- **Aplicar y restaurar:** Usa Optimizar en una tarjeta para aplicar su perfil y Restaurar para recuperar los ajustes anteriores guardados. Confirma los cambios antes de aplicarlos. El estado refleja los cambios gestionados por esta aplicación, no toda la configuración de Windows.
- **Perfiles modificados:** Si las opciones de CPU, GPU o Defender del ejecutable difieren del perfil aplicado, la tarjeta ofrece Reoptimizar. Tras confirmar, restaura los ajustes anteriores y aplica el perfil mostrado. Si falla el segundo paso, la optimización anterior ya no estará activa y puede requerirse recuperación. Si el perfil no tiene acciones habilitadas, usa Restaurar. Cambiar el nombre del perfil o los ajustes globales no requiere reoptimizar.
- **Valores predefinidos:** Esta acción elimina los ajustes de CPU y GPU seleccionados y descarta el respaldo correspondiente. Una vez completada, Restaurar no puede deshacerla. No elimina exclusiones de Defender; restaura primero una optimización que las incluya.
- **Defender:** Las exclusiones son opcionales y están desactivadas por defecto. Al activarlas se excluyen del análisis toda la carpeta del ejecutable y los archivos abiertos por ese proceso, lo que reduce la protección. La app comprueba que las exclusiones activas sigan coincidiendo; si no puede verificarlas, la tarjeta lo avisa sin modificarlas.
- **Optimización parcial:** Si no se puede aplicar un ajuste del perfil, la app informa el paso omitido e intenta los demás. Las exclusiones de carpeta y proceso de Defender se coordinan de forma independiente, por lo que un conflicto de carpeta no impide excluir el proceso. La advertencia muestra las rutas en conflicto cuando se pueden verificar; de lo contrario, indica que no se pudieron identificar. Si un paso fallido pudo modificar algo, la app intenta deshacerlo de inmediato y avisa si aún requiere recuperación. Restaurar solo deshace los ajustes aplicados y las recuperaciones pendientes. Las advertencias se muestran por separado, una a la vez.
- **Advertencias de entradas:** Una advertencia asociada a una entrada de la biblioteca puede ocultarse para esa entrada mediante su casilla. Para reactivar sus advertencias ocultas, usa «Volver a mostrar advertencias» en el menú de la entrada. Las demás entradas y los avisos dentro de la aplicación no se ven afectados. Si el comportamiento de inicio está configurado para cerrar Game Optimizer, se cierra al iniciar incluso cuando ocurre una advertencia.
- **Optimizaciones globales:** El plan de energía, Modo Juego y Game DVR están separados de los perfiles y no realizan cambios por defecto. El plan de energía afecta a todo el equipo y el plan elegido debe existir. Game DVR controla la grabación de Windows, no los gráficos del juego.
- **Configuración y actualizaciones:** Elige inglés o español y si la app se minimiza o cierra tras iniciar correctamente. En Configuración > Acerca de puedes buscar la última versión estable o abrir GitHub para reportar errores y sugerir mejoras. La búsqueda opcional al iniciar está desactivada por defecto y solo avisa. La descarga abre en el navegador el ejecutable portátil para Windows x64; reemplaza el anterior manualmente. La instalación y la verificación de firma no son automáticas.

Si un ajuste de Windows cambió por otro medio, Restaurar no sobrescribe ese cambio. Si una operación se interrumpe o necesita recuperación, conserva los datos de la app y la entrada de la biblioteca y usa Restaurar. Solo una instancia puede acceder a los datos de la app a la vez.

La versión publicada solicita permiso de administrador al iniciar, en lugar de pedirlo para cada acción de CPU o Defender. Inicia sesión con una cuenta administradora: no se admite introducir las credenciales de otra cuenta, porque se usarían sus datos y ajustes de Windows. Los juegos se inician con los permisos normales del usuario del escritorio; si no se pueden verificar, el inicio falla en lugar de ejecutarlos elevados. Es posible que los juegos que requieren administrador no se inicien de esta forma. Por restricciones de Windows, arrastrar archivos desde una ventana normal del Explorador hacia la app elevada puede no funcionar; usa Agregar ejecutable.

La apertura de carpetas y enlaces externos usa el shell de Windows desde la app elevada. Según la aplicación asociada y si ya está ejecutándose, podría iniciarse también con permisos de administrador.

Los [scripts anteriores](./legacy-scripts/) son independientes de la aplicación de escritorio y tienen otros comportamientos y riesgos.

## Verificar la descarga

Cada release incluye un archivo `.sha256` junto al ejecutable. Para comprobar que el archivo descargado coincide, ejecuta en PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 .\Game-Optimizer-<etiqueta>-windows-x64.exe
```

Compara el resultado con el hash del archivo `.sha256`. Esto confirma que la descarga no se corrompió ni se alteró en el camino; por sí solo no identifica al editor, porque el ejecutable todavía no está firmado digitalmente.

## Más documentación

- [Detalles de recuperación e implementación](./docs/recovery-ES.md): dónde se guardan los datos y las reservas, cómo funciona la restauración y sus límites conocidos.
- [Desarrollo](./docs/development-ES.md): compilación, publicación de releases y pruebas reales opcionales.
- [Política de privacidad](./docs/privacy-ES.md): qué envía la app por la red y qué guarda localmente.

## Licencia

Game Optimizer se distribuye bajo la Licencia Pública General de GNU, versión 3 o, a tu elección, cualquier versión posterior (GPL-3.0-or-later). Consulta [LICENSE](./LICENSE). Las dependencias de terceros conservan sus propias licencias.

