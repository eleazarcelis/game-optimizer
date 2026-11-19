# Privacy policy

[Español](./privacy-ES.md) | English

Game Optimizer does not collect, store remotely or transmit personal data, and it has no telemetry, analytics or accounts.

## Network use

- The only network request the app makes itself is to the GitHub API (`api.github.com`) to look up the latest stable release. It happens only when you press the update check in Settings > About, or on startup if you enable the optional startup check (off by default).
- That request goes directly from your computer to GitHub. GitHub receives the usual connection data (such as your IP address) and handles it under [GitHub's privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement). Game Optimizer sends no identifier of its own.
- Downloading an update, or reporting an issue, opens a GitHub page in your browser. What happens there is governed by your browser and GitHub.
- The app does not use any other network service.

## Data stored on your computer

- Your library, profiles, settings, the transaction journal and the backups used to restore previous settings are stored locally in the app's data folder and are never uploaded.
- Logs are written locally to the app's log folder and are never uploaded. If you report a bug, share them only if you choose to.

## What the app changes on your system

Game Optimizer changes selected Windows settings you ask it to change: CPU priority through the registry, GPU preference, optional power plan and Game Mode settings, and optional Microsoft Defender exclusions. It asks for confirmation first, records each change in its journal and can restore the previous values. Defender exclusions are off by default, apply only to the folder and process you choose, and reduce protection for those items while they are active.

## Third-party components

Game Optimizer is built from open-source dependencies. They keep their own licenses, and none of them is used to send data to other systems.

## Changes to this policy

Changes to this policy are published in this repository and recorded in its history.

## Contact

Questions or concerns: open an issue at [github.com/eleazarcelis/game-optimizer/issues](https://github.com/eleazarcelis/game-optimizer/issues).
