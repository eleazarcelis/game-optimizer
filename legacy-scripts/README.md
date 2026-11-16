# Legacy scripts

[Español](./README-ES.md) | English | [Desktop app](../README.md)

These older Windows scripts are separate from the desktop app. They change Windows settings for games, launchers and other processes; they do **not** change a game's visual settings. Their results depend on the system, and improvements to FPS, latency or compatibility are not guaranteed. Some messages printed by the scripts make stronger claims and should not be treated as guarantees.

> **Warning:** The scripts change system settings and may reduce antivirus protection. Their revert options do not generally restore every previous value, and can remove Defender exclusions that existed before using the scripts. Review and back up your settings before running them.

## What they do

- Both scripts offer launcher, game and secondary-process options. They request a Windows GPU preference and change Game Mode and the power plan. The game option also disables Game DVR recording and adds Defender exclusions for the executable and its entire folder.
- The Spanish script also changes CPU priority for future launcher or game processes with the same executable name, including in other folders. The English script does not change CPU priority.
- The power plan affects the entire computer. Windows or the app may ignore the requested GPU preference. Defender exclusions reduce protection for the affected folder and process.
- Revert removes the GPU preference and applies preset values to other settings; it does not restore all previous settings. The Spanish script separately backs up CPU priority and attempts to restore it; without a backup it leaves the current CPU value unchanged.

## Usage

1. On Windows, run `game-optimizer-en.bat` or `game-optimizer-es.bat`. Keep `cpu-priority.ps1` next to the Spanish script.
2. Select an option from 1–3 to configure or 4–6 to revert, then enter the full path to the executable.
3. Game configuration and reversion request administrator approval. The Spanish script also requires administrator permissions to configure or revert launcher and game CPU priority. Review any prompts and verify the resulting settings.

The scripts use PowerShell and write logs under `legacy-scripts\logs`. Their messages and revert options are not equivalent to the desktop app's transaction-based restoration.

## Implementation

GPU preferences are written under `HKCU\Software\Microsoft\DirectX\UserGpuPreferences` and `HKCU\System\GameConfigStore\Children`. The scripts also modify `AutoGameModeEnabled`, `GameDVR_Enabled`, power plans and Defender exclusions. The Spanish CPU helper uses Image File Execution Options and stores its backup in the machine registry. Administrator access is required for CPU changes and Defender exclusions. See `cpu-priority.ps1` and `tests\cpu-priority.tests.ps1` for the CPU implementation and tests.

## License

See the repository's [license](../LICENSE).
