# Game Optimizer

[Español](./README-ES.md) | English

Game Optimizer is a Windows desktop app for organizing games and managing how their processes run in Windows. It changes selected Windows settings; **it does not change a game's graphics settings**, such as resolution, visual quality or effects. Results depend on the game, hardware and Windows; improved performance or FPS is not guaranteed.

**Focus on stability rather than raw FPS.** In the author's tests with real games, the main benefit has been more consistent frame delivery, with less stuttering, rather than a higher average FPS. The settings it manages are meant to reduce interference from other processes and the system. The effect varies by game, hardware and Windows, and is not guaranteed.

> **Work in progress:** The app does not yet replace all features of the legacy scripts.

## Current features

- **Library:** Add Windows executables with the file picker and assign each a profile. Launch a game or open its folder from its card. Launching does not apply the profile automatically.
- **Steam detection:** The library search button detects Steam and installed Steam libraries; Settings > Automatic detection also offers separate Steam and game searches. Review results and select the correct executable before adding: game folders can contain installers or tools. Steam receives the Launcher profile and games the Game profile; existing entries are not duplicated or optimized. Linked folders and scan limits may leave results incomplete and are reported.
- **Organization:** Choose a grid or list, mark favorites, sort by name or arrange items manually. Related names can be grouped, although automatic grouping may not always match. Manual ordering also works with the keyboard; favorites stay first.
- **Profiles:** Under Settings > Profiles, adjust the built-in profiles or create your own. A profile can request a CPU priority and GPU preference for an executable. CPU priority applies to future processes with the same executable name, including in other folders, and requires administrator permission. The GPU preference asks Windows which GPU to use; Windows or the app may choose differently.
- **Process tree priority:** When launching a program from the library with a verified, applied CPU priority, Windows also applies that priority to compatible child processes using a Job Object, without a monitoring service. Already running processes and processes created outside the tree or through other services may be excluded; other Windows limits may also take precedence. If configuration fails, the program still launches and a warning is shown. The Job Object priority remains until the launched process tree exits, even if the profile is restored or Game Optimizer is closed; restore or change the profile before starting a new run. External launches continue to use only the executable-name setting.
- **Microsoft Store apps:** Executables in registered WindowsApps packages are activated through Windows. If a CPU priority is applied, the app warns that the process tree cannot be placed in its Job Object; the executable-name setting still applies. Packages with multiple possible app entries must have one matching the executable name.
- **Apply and restore:** Use Optimize on a library card to apply its profile, and Restore to return to the saved previous settings. Confirm changes before applying them. Status describes changes managed by this app, not all Windows settings.
- **Changed profiles:** When the executable's CPU, GPU or Defender choices differ from the applied profile, the card offers Re-optimize. After confirmation, it restores the previous settings and then applies the displayed profile. If the second step fails, the previous optimization is no longer active and recovery may be required. If the profile has no enabled actions, use Restore instead. Changes to a profile's name or global settings do not require re-optimization.
- **Defaults:** This action removes the selected CPU and GPU settings and discards the corresponding saved backup. Once completed, Restore cannot undo it. It does not remove Defender exclusions; restore an optimization that includes them first.
- **Defender:** Exclusions are optional and off by default. Enabling them excludes the executable's entire folder and files opened by that process from scanning, reducing protection. The app checks whether active exclusions still match; if it cannot verify them, the card reports this without changing them.
- **Partial optimization:** If an individual profile setting cannot be applied, the app reports the skipped step and attempts the others. Defender folder and process exclusions are coordinated independently, so a folder conflict does not prevent a process exclusion. The folder warning shows conflicting paths when they can be verified; otherwise it says they could not be identified. If a failed step may have changed something, the app attempts to undo it immediately and reports any recovery still needed. Restoration only undoes applied settings and any pending recovery. Warnings are shown separately, one at a time.
- **Entry warnings:** A warning associated with a library entry can be hidden for that entry using its checkbox. Use “Show warnings again” in the entry's menu to restore its hidden warnings. Other entries and in-app notices are unaffected. If launch behavior is set to close Game Optimizer, it closes after launch even when a warning occurs.
- **Global optimizations:** Power plan, Game Mode and Game DVR are separate from profiles and do nothing by default. The power plan affects the entire computer and the chosen plan must exist. Game DVR controls Windows recording, not game graphics.
- **Settings and updates:** Choose English or Spanish and whether the app minimizes or closes after a successful launch. Under Settings > About, check for the latest stable release on GitHub or open GitHub to report a bug or suggest an improvement. Optional startup checks are off by default and only notify you. Downloading opens the portable Windows x64 executable in your browser; replace the old executable manually. Installation and signature verification are not automatic.

If Windows settings were changed elsewhere, Restore will not overwrite the conflicting change. If an operation is interrupted or needs recovery, keep the app data and the library entry and use Restore. Only one instance can access the app data at a time.

The release app requests administrator approval at startup, rather than for each CPU or Defender action. Sign in with an administrator account: using another account's credentials at the prompt is not supported because it would use that account's data and Windows settings. Games start with the desktop user's non-elevated permissions; if that cannot be verified, launch fails instead of starting them elevated. Games that require administrator access may not start this way. Due to Windows restrictions, dragging files from a normal Explorer window into the elevated app may not work; use Add executable.

Opening folders and external links uses the Windows shell from the elevated app. Depending on the associated application and whether it is already running, it may also start elevated.

The [legacy scripts](./legacy-scripts/) are separate from the desktop app and have different behavior and risks.

## Verify your download

Each release includes a `.sha256` file next to the executable. To check that the file you downloaded matches it, run in PowerShell:

```powershell
Get-FileHash -Algorithm SHA256 .\Game-Optimizer-<tag>-windows-x64.exe
```

Compare the result with the hash in the `.sha256` file. This confirms the download was not corrupted or altered in transit; it does not by itself identify the publisher, because the executable is not yet code-signed.

## More documentation

- [Recovery and implementation details](./docs/recovery.md): where data and reservations are stored, how restoration works and known limits.
- [Development](./docs/development.md): building, publishing releases and the opt-in real tests.
- [Privacy policy](./docs/privacy.md): what the app sends over the network and what it stores locally.

## License

Game Optimizer is licensed under the GNU General Public License, version 3 or (at your option) any later version (GPL-3.0-or-later). See [LICENSE](./LICENSE). Third-party dependencies retain their own licenses.

