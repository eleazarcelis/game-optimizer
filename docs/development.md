# Development

[Español](./development-ES.md) | English

## Development

Requirements: Windows, Node.js, Rust with the MSVC toolchain, Visual C++ build tools and WebView2 Runtime.

If Cargo is installed through rustup but `cargo` is not found, restart VS Code or add it to the current PowerShell session:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo --version
```

If `cargo --version` still fails, install Rust with rustup and reopen the terminal.

```powershell
npm install
npm run tauri dev
```

Run frontend tests with `npm test` and native tests with `cargo test --manifest-path src-tauri\Cargo.toml`.
Debug builds run without a required elevation manifest and may prompt separately for privileged actions; the release build requires administrator approval at startup.

To build a portable executable without an installer:

```powershell
npm run tauri build -- --no-bundle
```

The executable is produced under `src-tauri\target\release`. WebView2 Runtime must be installed on the target machine. App data remains on the machine, not beside the executable.

## Publishing a release

Once the workflow is on the default branch, update the version in `package.json` and `package-lock.json`, then publish a GitHub release for a matching tag such as `v1.2.3` or `v1.2.3-rc.1` (build metadata such as `+build` is not supported). The workflow checks out that tag, verifies it matches `package.json`, builds a Windows x64 portable executable and attaches `Game-Optimizer-<tag>-windows-x64.exe`, its SHA-256 checksum file `Game-Optimizer-<tag>-windows-x64.exe.sha256` and `LICENSE` to the release. The hash is also shown in the workflow run summary. Tauri reads the displayed app version and Windows executable version metadata from `package.json`; each numeric version component must be at most 65535. Draft releases build when published. If any of these assets already exists, remove it before rerunning the workflow.

Pushes to `main` run the same build and populate the Rust dependency cache for later releases; they do not upload release assets. The first build, or a build after a cache miss, may still take longer.

## Real GPU recovery test

This opt-in test temporarily changes the current user's GPU preference **only for the generated dummy executable path**. It injects a failure after the write, then restores and verifies the previous registry value. The journal must not already exist. Keep the test directory and journal until restoration is confirmed.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerTest-$id"
New-Item -ItemType Directory -Path $dir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dir "dummy-$id.exe"
$env:GAME_OPTIMIZER_TEST_JOURNAL = Join-Path $dir 'journal.json'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_gpu_preference_recovers_after_a_partial_failure -- --ignored --nocapture
```

If interrupted or restoration fails, keep the same `GAME_OPTIMIZER_TEST_JOURNAL` and run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_dummy_gpu_journal -- --ignored --nocapture
```

## Real Defender test

This opt-in test requires several UAC approvals. It temporarily adds exclusions for a new folder and a disposable executable inside it, then attempts to restore them and verify the result. The folder exclusion affects all its files while active, so do not put other files there. The journal is stored separately.

```powershell
$id = [guid]::NewGuid().ToString('N')
$dir = Join-Path $env:TEMP "GameOptimizerDefenderTest-$id"
$dummyDir = Join-Path $dir 'dummy'
New-Item -ItemType Directory -Path $dummyDir | Out-Null
$env:GAME_OPTIMIZER_DUMMY_EXE = Join-Path $dummyDir "dummy-$id.exe"
$env:GAME_OPTIMIZER_SERVICE_ROOT = Join-Path $dir 'journal'
rustc --edition 2021 tests\fixtures\dummy.rs -o $env:GAME_OPTIMIZER_DUMMY_EXE
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the dummy executable' }
cargo build --manifest-path src-tauri\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'Failed to build the administrator helper' }
$env:GAME_OPTIMIZER_HELPER_EXE = (Resolve-Path src-tauri\target\debug\game-optimizer-app.exe).Path
cargo test --manifest-path src-tauri\Cargo.toml real_dummy_coordinated_defender_profile_applies_and_restores -- --ignored --nocapture
```

If the test is interrupted or restoration fails, keep the directory and journal. With the same `GAME_OPTIMIZER_DUMMY_EXE`, `GAME_OPTIMIZER_SERVICE_ROOT` and `GAME_OPTIMIZER_HELPER_EXE`, try to recover the journal:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml recover_full_profile -- --ignored --nocapture
```

Do not delete the directory or journal until the exclusions have been verified as restored.

