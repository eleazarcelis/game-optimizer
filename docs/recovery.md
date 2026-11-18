# Recovery and implementation details

[Español](./recovery-ES.md) | English

## Implementation and recovery details

The app is built with Tauri 2, React and TypeScript. Library, profiles and transaction journals are stored under the application's `%LOCALAPPDATA%` directory; logs use Tauri's Windows log directory. Replacing the portable executable on the same Windows account does not remove those files.

Defaults permanently removes enabled CPU/GPU settings, including those in an active optimization, and discards the corresponding backup. Interrupted operations retain recovery information. Older defaults transactions remain restorable or can be explicitly cleared with Defaults.

Defender restoration preserves preexisting exclusions and verifies changes with elevation. Overlapping folder exclusions and process exclusions for physically identical executables are coordinated independently, including across users. Older journals without resource identities require manual inspection. Do not move or redirect the executable's folders or links while applying or restoring an exclusion. Machine-wide reservations are stored in `%PROGRAMDATA%\GameOptimizer\resources` and new Defender reservations in `%PROGRAMDATA%\GameOptimizer-Defender-2739818f`; do not delete them while changes are active. Restore older Defender transactions before creating new exclusions. These reservations coordinate this app, but Windows or other programs may change Defender independently. Restore independent exclusions with this version or newer, not an older build.

If a restoration skips externally changed Defender exclusions, the app releases its reservation without changing those exclusions. Before applying another Defender profile, it also checks completed journals from older versions and releases matching leftover reservations only when both exclusions are absent and their resource identities still match.

Global actions update `AutoGameModeEnabled` and `GameDVR_Enabled` for the current Windows user; their effectiveness depends on Windows and policy settings. The previous registry values and types are journaled before each write. After interruption, use Restore in global settings; external changes are not overwritten.

## Pending work

Defender resource snapshots record the Windows volume and file ID. CPU restoration restores the priority value but can leave empty IFEO registry keys created during application. Comprehensive UI and interruption testing is still needed.
