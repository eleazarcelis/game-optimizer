# Game Optimizer

Aplicación de escritorio para Windows construida con Tauri 2, React y TypeScript.

> **Trabajo en curso:** se pueden aplicar y restaurar la prioridad de CPU, la preferencia de GPU, las exclusiones opcionales de Defender y los ajustes globales; valores predefinidos elimina los ajustes de CPU y GPU seleccionados de forma permanente. Todavía no reemplaza completamente los scripts anteriores.

## Funcionalidades actuales

- Interfaz en inglés y español.
- Biblioteca en grilla con iconos de Windows y nombres de metadatos, favoritos primero, orden A–Z/Z–A o manual y agrupamiento opcional y colapsable por nombres relacionados. La agrupación usa una heurística de prefijos y puede no acertar siempre. El orden manual se guarda; los controles de arrastre también admiten las flechas del teclado. El aviso de la biblioteca se puede cerrar permanentemente.
- En orden manual se puede arrastrar la tarjeta o usar su control con el teclado. Al agrupar, los grupos con favoritos aparecen primero y sus favoritos se muestran al inicio. El orden manual respeta esa prioridad y permite ordenar dentro de un grupo o mover el grupo completo desde su control.
- Al arrastrar, una línea marca la posición exacta de inserción; no se muestra si la posición no cambia el orden o no está permitida.
- Ventana con navegación y encabezados fijos: solo se desplaza el contenido de los paneles, con barras de desplazamiento de estilo oscuro/violeta y acceso por teclado.
- Agregar ejecutables mediante el selector o arrastrándolos; se asigna un perfil al agregarlos.
- Abrir la carpeta de un ejecutable o ejecutarlo mediante su botón o icono. En Configuración se elige si la app se minimiza (por defecto) o se cierra tras iniciar correctamente; ejecutar no aplica el perfil automáticamente.
- Administrar el idioma y el comportamiento al ejecutar en Configuración > General, y los perfiles predeterminados y personalizados en Configuración > Perfiles.
- Aplicar y restaurar prioridad de CPU y preferencias de GPU con confirmación y diario persistente. La prioridad de CPU requiere permiso de administrador y afecta a procesos futuros con el mismo nombre de ejecutable, también en otras carpetas.
- Valores predefinidos elimina permanentemente los ajustes habilitados de CPU y GPU y las acciones de CPU/GPU de una optimización activa. Descarta el respaldo de esa optimización y, una vez completado, Restaurar no puede deshacerlo. Si se interrumpe, se conserva información para su recuperación. Las transacciones antiguas de valores predefinidos siguen pudiéndose restaurar o limpiar explícitamente.
- Las opciones globales de energía, Modo Juego y Game DVR están separadas de los perfiles y, por defecto, no realizan cambios. El plan de energía afecta a todo el equipo y el plan seleccionado debe existir.
- Las exclusiones de Defender son opcionales y están desactivadas por defecto. Al optimizar se excluyen la carpeta completa del ejecutable y los archivos abiertos por ese proceso, lo que reduce la protección antivirus; requiere aprobaciones de administrador. Se rechazan perfiles activos con carpetas superpuestas o ejecutables que comparten identidad física, incluso entre usuarios. Restaurar conserva las exclusiones preexistentes y verifica los cambios con elevación. El estado se muestra como no verificado hasta restaurar. Valores predefinidos no modifica Defender: restaura primero una optimización que lo incluya. Los diarios antiguos sin identidad del recurso requieren inspección manual.
- La biblioteca y los perfiles se guardan en la carpeta de la aplicación dentro de `%LOCALAPPDATA%`. Los logs utilizan el directorio de registros de Tauri para Windows.

El estado del perfil refleja los cambios gestionados por la aplicación, no una evaluación de toda la configuración de Windows. Si se detecta que un ajuste cambió externamente, la restauración se niega a sobrescribirlo; resuelve el conflicto antes de reintentar. No elimines los datos de la aplicación ni una entrada con cambios pendientes de recuperación. Solo una instancia puede acceder a los datos a la vez. Las reservas del equipo se guardan en `%PROGRAMDATA%\GameOptimizer\resources` y las nuevas reservas de Defender en `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; no las elimines mientras haya cambios activos. Restaura las transacciones antiguas de Defender antes de crear nuevas exclusiones.

No muevas ni redirijas las carpetas o enlaces del ejecutable mientras se aplica o restaura una exclusión. Las reservas coordinan esta aplicación, pero Windows u otros programas pueden modificar Defender por separado.

Las acciones globales modifican `AutoGameModeEnabled` y `GameDVR_Enabled` del usuario actual de Windows; su efectividad depende de Windows y de las directivas existentes. El diario guarda los valores y tipos anteriores antes de escribir. Tras una interrupción, usa Restaurar en ajustes globales; los cambios externos no se sobrescriben.

Los scripts anteriores, su documentación y sus pruebas están en [legacy-scripts](./legacy-scripts/).

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

Para generar un ejecutable portable sin instalador:

```powershell
npm run tauri build -- --no-bundle
```

El ejecutable se genera en `src-tauri\target\release`. WebView2 Runtime debe estar instalado en el equipo de destino. Los datos permanecen en el equipo, no junto al ejecutable.

## Publicar una release

Cuando el workflow esté en la rama predeterminada, actualiza la versión en `package.json` y `package-lock.json` y publica una release de GitHub con una etiqueta coincidente como `v1.2.3` o `v1.2.3-rc.1` (no se admiten metadatos de compilación como `+build`). El workflow descarga esa etiqueta, verifica que coincida con `package.json`, compila un ejecutable portable para Windows x64 y adjunta `Game-Optimizer-<etiqueta>-windows-x64.exe` a la release. Tauri lee de `package.json` la versión mostrada en la app y los metadatos de versión del ejecutable; cada componente numérico debe ser como máximo 65535. Las releases en borrador se compilan al publicarlas. Si ya existe un archivo con ese nombre, elimínalo antes de volver a ejecutar el workflow.

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
