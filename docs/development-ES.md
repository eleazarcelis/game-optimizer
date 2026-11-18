# Desarrollo

Español | [English](./development.md)

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

Cuando el workflow esté en la rama predeterminada, actualiza la versión en `package.json` y `package-lock.json` y publica una release de GitHub con una etiqueta coincidente como `v1.2.3` o `v1.2.3-rc.1` (no se admiten metadatos de compilación como `+build`). El workflow descarga esa etiqueta, verifica que coincida con `package.json`, compila un ejecutable portable para Windows x64 y adjunta `Game-Optimizer-<etiqueta>-windows-x64.exe`, su archivo de suma de verificación SHA-256 `Game-Optimizer-<etiqueta>-windows-x64.exe.sha256` y `LICENSE` a la release. El hash también aparece en el resumen de la ejecución del workflow. Tauri lee de `package.json` la versión mostrada en la app y los metadatos de versión del ejecutable; cada componente numérico debe ser como máximo 65535. Las releases en borrador se compilan al publicarlas. Si alguno de esos archivos ya existe, elimínalo antes de volver a ejecutar el workflow.

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

