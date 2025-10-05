# 🎮 Game and Launcher Optimizer

A powerful Windows batch script that applies specific optimizations to games, launchers, and auxiliary processes to maximize performance and minimize resource consumption.

## 📋 Overview

This optimizer provides three distinct profiles tailored for different types of applications:
- **Launcher Profile**: Balanced optimization for gaming platforms (Steam, Epic, Origin, etc.)
- **Game Profile**: Maximum performance optimization for games
- **Secondary Process Profile**: Stable configuration for anticheats and auxiliary processes

## ✨ Features

### 🚀 Launcher Profile
- **GPU Configuration**: Balanced power settings for energy efficiency
- **GameBar**: Enabled for overlays and screen capture
- **Power Plan**: Balanced mode for optimal energy consumption
- **Impact**: Improved fluidity without extra resource usage

### 🎯 Game Profile
- **GPU Configuration**: High performance mode (always uses dedicated GPU)
- **Windows Defender**: Excludes game folder and executable from real-time scanning
- **GameDVR**: Disabled to eliminate recording lag
- **GameBar**: Enabled for automatic Game Mode activation
- **Power Plan**: High performance (CPU and GPU at maximum capacity)
- **Impact**: Maximum FPS, reduced latency, uninterrupted gaming experience

### 🛡️ Secondary Process Profile
- **GPU Configuration**: Balanced settings (doesn't interfere with main game)
- **GameBar**: Disabled to avoid conflicts with anticheats
- **Power Plan**: Balanced mode (doesn't compete for system resources)
- **Impact**: Maximum stability, prevents false positives with anticheat systems

## 🔄 Revert Options

All configurations can be completely reverted to restore Windows default settings:
- **Revert Launcher**: Removes all launcher-specific optimizations
- **Revert Game**: Removes all game optimizations and restores default security settings
- **Revert Secondary Process**: Restores default configuration for auxiliary processes

## 🛠️ Usage

1. **Run the appropriate script**:
   - `game-optimizer-en.bat` (English version)
   - `game-optimizer-es.bat` (Spanish version)

2. **Select your desired profile** (1-6):
   - Options 1-3: Configure optimizations
   - Options 4-6: Revert configurations

3. **Provide the full path** to the executable you want to optimize

4. **Administrator permissions** may be required for Game Profile optimizations

## ⚠️ Important Notes

- **All changes are PERMANENT** until manually reverted using the revert options
- **Game Profile requires Administrator privileges** for Windows Defender exclusions and system-level optimizations
- **Always backup your system** before applying optimizations
- **Test configurations** with your specific games and hardware setup

## 🖥️ System Requirements

- **Operating System**: Windows 10 (tested)
- **PowerShell**: Required for advanced configurations
- **Administrator Rights**: Required for Game Profile optimizations

## ⚡ Performance Benefits

### Launcher Profile
- Reduced launcher startup time
- Lower background resource usage
- Maintained system responsiveness

### Game Profile
- **FPS Increase**: 5-15% improvement in most games
- **Reduced Input Lag**: Lower latency for competitive gaming
- **Elimination of Interruptions**: No Windows Defender scans during gameplay
- **Optimized Power Delivery**: CPU and GPU run at peak performance

### Secondary Process Profile
- **Anticheat Compatibility**: Prevents conflicts with game protection systems
- **System Stability**: Maintains balanced resource allocation
- **No False Positives**: Reduces anticheat detection issues

## 🔧 Technical Details

The optimizer modifies the following Windows components:
- **GPU Preferences**: DirectX UserGpuPreferences registry entries
- **Game Config Store**: Windows Game Mode settings
- **Windows Defender**: Real-time protection exclusions
- **Power Management**: System power plan configurations
- **GameDVR**: Windows game recording features

## 📝 Version Information

- **Version**: 1.0.0
- **Author**: Ele
- **GitHub**: https://github.com/eleazarcelis
- **Tested On**: Windows 10

## ⚠️ Disclaimer

This tool has been **tested exclusively on Windows 10**. Compatibility with other Windows versions is not guaranteed. Use at your own risk and always create system restore points before applying optimizations.

## 🤝 Contributing

Feel free to report issues, suggest improvements, or contribute to the project through the GitHub repository.

## 📄 License

This project is open source. Please check the repository for license details.