# Game Optimizer

Español | [English](./README.md)

Game Optimizer es una aplicación de escritorio para Windows que organiza juegos y ayuda a gestionar cómo se ejecutan sus procesos en Windows. Modifica ajustes seleccionados del sistema; **no cambia la configuración gráfica del juego**, como la resolución, la calidad visual o los efectos. Los resultados dependen del juego, el equipo y Windows; no se garantiza una mejora de rendimiento o FPS.

> **Trabajo en curso:** la aplicación todavía no reemplaza todas las funciones de los scripts anteriores.

## Funcionalidades actuales

- **Biblioteca:** Agrega ejecutables de Windows con el selector y asigna un perfil a cada uno. Desde su tarjeta puedes iniciar un juego o abrir su carpeta. Iniciarlo no aplica el perfil automáticamente.
- **Organización:** Elige una grilla o lista, marca favoritos, ordena por nombre o acomoda los elementos manualmente. Puedes agrupar nombres relacionados, aunque la agrupación automática puede equivocarse. El orden manual también admite el teclado; los favoritos permanecen primero.
- **Perfiles:** En Configuración > Perfiles, ajusta los perfiles incluidos o crea los tuyos. Un perfil puede solicitar una prioridad de CPU y una preferencia de GPU para un ejecutable. La prioridad de CPU afecta a futuras ejecuciones con el mismo nombre, incluso en otras carpetas, y requiere permiso de administrador. La preferencia de GPU solicita a Windows qué GPU usar; Windows o la aplicación pueden elegir otra.
- **Aplicar y restaurar:** Usa Optimizar en una tarjeta para aplicar su perfil y Restaurar para recuperar los ajustes anteriores guardados. Confirma los cambios antes de aplicarlos. El estado refleja los cambios gestionados por esta aplicación, no toda la configuración de Windows.
- **Perfiles modificados:** Si las opciones de CPU, GPU o Defender del ejecutable difieren del perfil aplicado, la tarjeta ofrece Reoptimizar. Tras confirmar, restaura los ajustes anteriores y aplica el perfil mostrado. Si falla el segundo paso, la optimización anterior ya no estará activa y puede requerirse recuperación. Si el perfil no tiene acciones habilitadas, usa Restaurar. Cambiar el nombre del perfil o los ajustes globales no requiere reoptimizar.
- **Valores predefinidos:** Esta acción elimina los ajustes de CPU y GPU seleccionados y descarta el respaldo correspondiente. Una vez completada, Restaurar no puede deshacerla. No elimina exclusiones de Defender; restaura primero una optimización que las incluya.
- **Defender:** Las exclusiones son opcionales y están desactivadas por defecto. Al activarlas se excluyen del análisis toda la carpeta del ejecutable y los archivos abiertos por ese proceso, lo que reduce la protección. La app comprueba que las exclusiones activas sigan coincidiendo; si no puede verificarlas, la tarjeta lo avisa sin modificarlas.
- **Optimizaciones globales:** El plan de energía, Modo Juego y Game DVR están separados de los perfiles y no realizan cambios por defecto. El plan de energía afecta a todo el equipo y el plan elegido debe existir. Game DVR controla la grabación de Windows, no los gráficos del juego.
- **Configuración y actualizaciones:** Elige inglés o español y si la app se minimiza o cierra tras iniciar correctamente. En Configuración > Acerca de puedes buscar la última versión estable o abrir GitHub para reportar errores y sugerir mejoras. La búsqueda opcional al iniciar está desactivada por defecto y solo avisa. La descarga abre en el navegador el ejecutable portátil para Windows x64; reemplaza el anterior manualmente. La instalación y la verificación de firma no son automáticas.

Si un ajuste de Windows cambió por otro medio, Restaurar no sobrescribe ese cambio. Si una operación se interrumpe o necesita recuperación, conserva los datos de la app y la entrada de la biblioteca y usa Restaurar. Solo una instancia puede acceder a los datos de la app a la vez.

La versión publicada solicita permiso de administrador al iniciar, en lugar de pedirlo para cada acción de CPU o Defender. Inicia sesión con una cuenta administradora: no se admite introducir las credenciales de otra cuenta, porque se usarían sus datos y ajustes de Windows. Los juegos se inician con los permisos normales del usuario del escritorio; si no se pueden verificar, el inicio falla en lugar de ejecutarlos elevados. Es posible que los juegos que requieren administrador no se inicien de esta forma. Por restricciones de Windows, arrastrar archivos desde una ventana normal del Explorador hacia la app elevada puede no funcionar; usa Agregar ejecutable.

La apertura de carpetas y enlaces externos usa el shell de Windows desde la app elevada. Según la aplicación asociada y si ya está ejecutándose, podría iniciarse también con permisos de administrador.

Los [scripts anteriores](./legacy-scripts/) son independientes de la aplicación de escritorio y tienen otros comportamientos y riesgos.

## Detalles de implementación y recuperación

La app está construida con Tauri 2, React y TypeScript. La biblioteca, los perfiles y los diarios de transacciones se guardan en la carpeta de la aplicación dentro de `%LOCALAPPDATA%`; los logs usan el directorio de registros de Tauri para Windows. Reemplazar el ejecutable portátil en la misma cuenta de Windows no elimina esos archivos.

Valores predefinidos elimina permanentemente los ajustes de CPU/GPU habilitados, incluidos los de una optimización activa, y descarta el respaldo correspondiente. Si se interrumpe, se conserva información para su recuperación. Las transacciones antiguas de valores predefinidos siguen pudiéndose restaurar o limpiar explícitamente.

La restauración de Defender conserva las exclusiones preexistentes y verifica los cambios con elevación. Se rechazan perfiles activos con carpetas superpuestas o ejecutables con la misma identidad física, incluso entre usuarios. Los diarios antiguos sin identidad del recurso requieren inspección manual. No muevas ni redirijas las carpetas o enlaces del ejecutable mientras se aplica o restaura una exclusión. Las reservas del equipo se guardan en `%PROGRAMDATA%\GameOptimizer\resources` y las nuevas reservas de Defender en `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; no las elimines mientras haya cambios activos. Restaura las transacciones antiguas de Defender antes de crear nuevas exclusiones. Estas reservas coordinan la aplicación, pero Windows u otros programas pueden modificar Defender por separado.

Las acciones globales modifican `AutoGameModeEnabled` y `GameDVR_Enabled` del usuario actual de Windows; su efectividad depende de Windows y de las directivas existentes. El diario guarda los valores y tipos anteriores antes de escribir. Tras una interrupción, usa Restaurar en ajustes globales; los cambios externos no se sobrescriben.

## Licencia

Game Optimizer se distribuye bajo la Licencia Pública General de GNU, versión 3 o, a tu elección, cualquier versión posterior (GPL-3.0-or-later). Consulta [LICENSE](./LICENSE). Las dependencias de terceros conservan sus propias licencias.

## Desarrollo

Requisitos: Windows, Node.js, Rust con toolchain MSVC, herramientas de compilación de Visual C++ y WebView2 Runtime.

Si Cargo está instalado mediante rustup pero no se encuentra `cargo`, reinicia VS Code o añádelo a la sesión actual de PowerShell:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo --version
```

Si `cargo --version` sigue fallando, instala Rust con rustup y vuelve a abrir la terminal.

```powershell
npm install
npm run tauri dev
```

Para las pruebas del frontend ejecuta `npm test`; para las pruebas nativas, `cargo test --manifest-path src-tauri\Cargo.toml`.
Las compilaciones de desarrollo no exigen elevación al iniciar y pueden solicitar permisos por separado para acciones privilegiadas; la versión de producción solicita administrador al iniciar.

Para generar un ejecutable portable sin instalador:

```powershell
npm run tauri build -- --no-bundle
```

El ejecutable se genera en `src-tauri\target\release`. WebView2 Runtime debe estar instalado en el equipo de destino. Los datos permanecen en el equipo, no junto al ejecutable.

## Publicar una release

Cuando el workflow esté en la rama predeterminada, actualiza la versión en `package.json` y `package-lock.json` y publica una release de GitHub con una etiqueta coincidente como `v1.2.3` o `v1.2.3-rc.1` (no se admiten metadatos de compilación como `+build`). El workflow descarga esa etiqueta, verifica que coincida con `package.json`, compila un ejecutable portable para Windows x64 y adjunta `Game-Optimizer-<etiqueta>-windows-x64.exe` y `LICENSE` a la release. Tauri lee de `package.json` la versión mostrada en la app y los metadatos de versión del ejecutable; cada componente numérico debe ser como máximo 65535. Las releases en borrador se compilan al publicarlas. Si alguno de esos archivos ya existe, elimínalo antes de volver a ejecutar el workflow.

Los pushes a `main` ejecutan la misma compilación y preparan la caché de dependencias de Rust para releases posteriores; no adjuntan archivos a ninguna release. La primera compilación o una compilación sin caché puede tardar más.

## Prueba real de recuperación de GPU

Esta prueba opcional cambia temporalmente la preferencia GPU del usuario actual **solo para la ruta del ejecutable descartable generado**. Simula un fallo después de escribir y luego restaura y verifica el valor anterior del registro. El archivo de registro no debe existir previamente. Conserva el directorio y el registro hasta comprobar la restauración.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerTest-$id"
New-Item -ItemType Directory -Path $dir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dir "dummy-$id.exe"
$env:GAME_OPTIMIZER_TEST_JOURNAL = Join-Path $dir 'journal.json'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_gpu_preference_recovers_after_a_partial_failure -- --ignored --nocapture
```

Si se interrumpe o falla la restauración, conserva el mismo `GAME_OPTIMIZER_TEST_JOURNAL` y ejecuta:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_dummy_gpu_journal -- --ignored --nocapture
```

## Prueba real de Defender

Esta prueba opcional requiere varias aprobaciones de UAC. Agrega temporalmente exclusiones para una carpeta nueva y un ejecutable descartable dentro de ella; después intenta restaurarlas y verifica el resultado. La exclusión de carpeta afecta a todos sus archivos mientras está activa, así que no guardes otros archivos allí. El diario se guarda en una carpeta separada.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerDefenderTest-$id"
$dummyDir = Join-Path $dir 'dummy'
New-Item -ItemType Directory -Path $dummyDir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dummyDir "dummy-$id.exe"
$env:GAME_OPTIMIZER_SERVICE_ROOT = Join-Path $dir 'journal'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
if ($LASTEXITCODE -ne 0) { throw 'No se pudo compilar el ejecutable de prueba' }
cargo build --manifest-path src-tauri\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'No se pudo compilar el asistente de administrador' }
$env:GAME_OPTIMIZER_HELPER_EXE = (Resolve-Path src-tauri\target\debug\game-optimizer-app.exe).Path
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_coordinated_defender_profile_applies_and_restores -- --ignored --nocapture
```

Si la prueba se interrumpe o falla la restauración, conserva el directorio y el diario. Con los mismos valores de `GAME_OPTIMIZER_DUMMY_EXE`, `GAME_OPTIMIZER_SERVICE_ROOT` y `GAME_OPTIMIZER_HELPER_EXE`, intenta recuperar el diario:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_full_profile -- --ignored --nocapture
```

No elimines el directorio ni el diario hasta comprobar que las exclusiones se restauraron.

## Pendiente

Las instantáneas de Defender registran el volumen y el identificador de archivo de Windows. La restauración de CPU recupera el valor de prioridad, pero puede dejar claves IFEO vacías creadas al aplicar el ajuste. También faltan pruebas exhaustivas de interfaz e interrupciones.
