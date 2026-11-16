# Game Optimizer

A Windows desktop application built with Tauri 2, React and TypeScript.

> **Work in progress:** CPU priority, GPU preference, optional Defender exclusions and global settings can be applied and restored; defaults removes selected CPU and GPU settings permanently. This is not yet a complete replacement for the legacy scripts.

## Current features

- English and Spanish UI.
- Grid library with Windows icons and executable metadata names, favorites first, A–Z/Z–A or manual ordering, and optional collapsible groups of related names. Grouping uses a name-prefix heuristic and may not always match correctly. Manual order is saved; drag handles also support arrow keys. The library notice can be dismissed permanently.
- In manual mode, drag a card or use its keyboard handle. With grouping enabled, groups containing favorites come first and favorite members appear first within their group. Manual reordering stays within the favorite/non-favorite priority and within a group, while group handles move whole groups.
- While dragging, a line marks the exact insertion position; it is hidden for positions that would not change the order or are not allowed.
- Fixed navigation and page headers: only panel contents scroll, with dark/purple scrollbars and keyboard access.
- Add executables using the file picker or drag and drop; assign a profile when adding.
- Open an executable's folder or launch it with its button or icon. In Settings, choose whether the app minimizes (default) or closes after a successful launch; starting a game does not automatically apply a profile.
- Manage language and launch behavior under Settings > General, built-in and custom profiles under Settings > Profiles, and view the app version under Settings > About.
- Settings > About can check GitHub for the latest stable release. Automatic checks are off by default; when enabled, the app checks at startup and notifies you without downloading. A separate button opens the Windows x64 portable executable download in your browser when available; replace the old executable manually. No automatic installation or signature verification is provided.
- Apply and restore CPU priority and GPU preferences with confirmation and a persistent transaction journal. CPU priority requires administrator approval and affects future processes with the same executable name, including in other folders.
- Defaults permanently removes the enabled CPU and GPU settings and any CPU/GPU actions in an active optimization. It discards that optimization's backup; once completed, Restore cannot undo it. Interrupted operations retain recovery information. Older defaults transactions remain restorable or can be explicitly cleared with Defaults.
- Global power, Game Mode and Game DVR options are separate from executable profiles and default to no change. Power changes affect the whole computer and require the selected plan to exist.
- Defender exclusions are optional and disabled by default. Optimization excludes the executable's entire folder and files opened by that process, reducing antivirus protection; administrator approvals are required. Active profiles with overlapping folders or physically identical executables are rejected, including across users. Restore preserves preexisting exclusions and verifies changes with elevation. Status remains unverified until restoration. Defaults does not change Defender: restore an optimization that includes it first. Older journals without resource identities require manual inspection.
- Library and profile data are stored under the application's `%LOCALAPPDATA%` directory. Application logs use Tauri's Windows log directory.

The profile status reflects changes managed by this application, not an assessment of all existing Windows settings. If an external setting change is detected, restoration refuses to overwrite it; resolve the conflict before retrying. Do not delete the application data or an active executable entry while its change requires recovery. Only one instance can access the application data at a time. Machine-wide reservations are stored in `%PROGRAMDATA%\GameOptimizer\resources` and new Defender reservations in `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; do not delete them while changes are active. Restore older Defender transactions before creating new exclusions.

Do not move or redirect the executable's folders or links while applying or restoring an exclusion. Reservations coordinate this application, but Windows or other programs can change Defender independently.

Global actions update `AutoGameModeEnabled` and `GameDVR_Enabled` for the current Windows user; their effectiveness depends on Windows and policy settings. The previous registry values and types are journaled before each write. If interrupted, use Restore in the global settings page; external changes are not overwritten.

The legacy scripts, their documentation and tests are in [legacy-scripts](./legacy-scripts/).

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

To build a portable executable without an installer:

```powershell
npm run tauri build -- --no-bundle
```

The executable is produced under `src-tauri\target\release`. WebView2 Runtime must be installed on the target machine. App data remains on the machine, not beside the executable.

## Publishing a release

Once the workflow is on the default branch, update the version in `package.json` and `package-lock.json`, then publish a GitHub release for a matching tag such as `v1.2.3` or `v1.2.3-rc.1` (build metadata such as `+build` is not supported). The workflow checks out that tag, verifies it matches `package.json`, builds a Windows x64 portable executable and attaches `Game-Optimizer-<tag>-windows-x64.exe` to the release. Tauri reads the displayed app version and Windows executable version metadata from `package.json`; each numeric version component must be at most 65535. Draft releases build when published. If an asset with that name already exists, remove it before rerunning the workflow.

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

## Pending work

Defender resource snapshots record the Windows volume and file ID. CPU restoration restores the priority value but can leave empty IFEO registry keys created during application. Comprehensive UI and interruption testing is still needed.
