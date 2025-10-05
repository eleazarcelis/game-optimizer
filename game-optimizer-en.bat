@echo off
setlocal EnableDelayedExpansion
title Game and Launcher Optimizer

echo ==============================================
echo  GAME AND LAUNCHER OPTIMIZER
echo ==============================================
echo.
echo Author: Ele
echo GitHub: https://github.com/eleazarcelis
echo Version: 1.0.0
echo ==============================================
echo.
echo This script optimizes applications by applying specific configurations
echo based on process type. Changes are PERMANENT until reverted.
echo.
echo ==============================================
echo  PROFILE EXPLANATIONS
echo ==============================================
echo.
echo [1] LAUNCHER - Basic optimization for gaming platforms:
echo     ^* GPU: Balanced configuration (saves energy)
echo     ^* GameBar: Enabled (allows overlays and captures)
echo     ^* Power plan: Balanced (energy efficiency)
echo     ^* IMPACT: Improves fluidity without extra resource consumption
echo.
echo [2] GAME - Maximum optimization for performance:
echo     ^* GPU: High performance (always uses dedicated GPU)
echo     ^* Windows Defender: Excludes folder and executable from scanning
echo     ^* GameDVR: Disabled (eliminates recording lag)
echo     ^* GameBar: Enabled for automatic game mode
echo     ^* Power plan: High performance (CPU and GPU at maximum)
echo     ^* IMPACT: Maximum FPS, lower latency, no interruptions
echo.
echo [3] SECONDARY PROCESS - Configuration for anticheats/auxiliaries:
echo     ^* GPU: Balanced configuration (doesn't interfere with game)
echo     ^* GameBar: Disabled (avoids conflicts with anticheats)
echo     ^* Power plan: Balanced (doesn't compete for resources)
echo     ^* IMPACT: Maximum stability, no false positives
echo.
echo [4-6] REVERT OPTIONS: Remove ALL applied configurations
echo       and restore system default values.
echo.
echo ==============================================
echo  SELECT AN OPTION
echo ==============================================
echo.
echo CONFIGURE:
echo [1] LAUNCHER (Steam, Epic, Origin, etc.)
echo [2] GAME (Complete optimization)
echo [3] SECONDARY PROCESS (Anticheats, etc.)
echo.
echo REVERT:
echo [4] REVERT LAUNCHER
echo [5] REVERT GAME
echo [6] REVERT SECONDARY PROCESS
echo.

set /p "profile=Select (1, 2, 3, 4, 5 or 6): "

if "%profile%"=="1" (
    set "type=LAUNCHER"
    set "action=CONFIGURE"
) else if "%profile%"=="2" (
    set "type=GAME"
    set "action=CONFIGURE"
) else if "%profile%"=="3" (
    set "type=SECONDARY PROCESS"
    set "action=CONFIGURE"
) else if "%profile%"=="4" (
    set "type=LAUNCHER"
    set "action=REVERT"
) else if "%profile%"=="5" (
    set "type=GAME"
    set "action=REVERT"
) else if "%profile%"=="6" (
    set "type=SECONDARY PROCESS"
    set "action=REVERT"
) else (
    echo Error: Invalid option
    pause
    exit /b 1
)

echo.
echo ==============================================
echo  %action% %type%
echo ==============================================
echo.

set /p "exe=Full path to executable: "
set "exe=%exe:"=%"

if "%exe%"=="" (
    echo Error: No path entered
    pause
    exit /b 1
)

if not exist "%exe%" (
    echo Error: File not found
    pause  
    exit /b 1
)

for %%I in ("%exe%") do (
    set "folder=%%~dpI"
    set "exename=%%~nxI"
)
if "%folder:~-1%"=="\" set "folder=%folder:~0,-1%"

echo.
echo Executable: "%exe%"
echo Folder: "%folder%"
echo Name: "%exename%"
echo.

:: =====================================================
:: Configure/Revert GPU
:: =====================================================
if "%action%"=="CONFIGURE" (
    echo [*] Configuring GPU preference...
    
    if "%profile%"=="1" (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] Balanced GPU configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    ) else if "%profile%"=="2" (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=2;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 2 -Force; Write-Host '[OK] High Performance GPU configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    ) else (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] Balanced GPU (Secondary Process) configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    )
) else (
    echo [*] Reverting GPU preference...
    
    powershell -NoProfile -Command "try { $exePath = '%exe%'; Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Force -ErrorAction SilentlyContinue; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; Remove-Item -Path $regPath -Recurse -Force -ErrorAction SilentlyContinue; Write-Host '[OK] GPU configuration removed' } catch { Write-Host '[!] Error reverting GPU' }"
)

:: Apply/Revert Optimizations
echo.
if "%action%"=="CONFIGURE" (
    echo [*] Applying optimizations...
    
    if "%profile%"=="1" (
        :: LAUNCHER PROFILE - CONFIGURE
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Basic configuration applied
    ) else if "%profile%"=="2" (
        :: GAME PROFILE - CONFIGURE
        goto :configure_game
    ) else (
        :: SECONDARY PROCESS PROFILE - CONFIGURE
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Secondary process configuration applied
    )
) else (
    echo [*] Reverting optimizations...
    
    if "%profile%"=="4" (
        :: LAUNCHER PROFILE - REVERT
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Launcher configuration reverted
    ) else if "%profile%"=="5" (
        :: GAME PROFILE - REVERT
        goto :revert_game
    ) else (
        :: SECONDARY PROCESS PROFILE - REVERT
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Secondary process configuration reverted
    )
)
goto :end_script

:configure_game
    :: GAME PROFILE
    echo [*] Creating advanced optimizations...
    
    set "ps1=%temp%\gameopt.ps1"
    
    (
        echo Write-Host "[*] Optimizing for game..."
        echo try {
        echo   Add-MpPreference -ExclusionPath "!folder!"
        echo   Add-MpPreference -ExclusionProcess "!exe!"
        echo   Write-Host "[OK] Defender exclusions"
        echo } catch { Write-Host "[!] Defender error" }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 0 -Force
        echo   Write-Host "[OK] GameDVR disabled"
        echo   powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
        echo   Write-Host "[OK] High performance"
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 1 -Force
        echo   Write-Host "[OK] Game mode activated"
        echo } catch { Write-Host "[!] Configuration error" }

        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    echo [*] Requesting administrator permissions...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    goto :end_script

:revert_game
    echo [*] Creating reversion script...
    
    set "ps1=%temp%\gamerevert.ps1"
    
    (
        echo Write-Host "[*] Reverting game optimizations..."
        echo try {
        echo   Remove-MpPreference -ExclusionPath "!folder!" -ErrorAction SilentlyContinue
        echo   Remove-MpPreference -ExclusionProcess "!exe!" -ErrorAction SilentlyContinue
        echo   Write-Host "[OK] Defender exclusions removed"
        echo } catch { Write-Host "[!] Error reverting Defender" }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 1 -Force
        echo   Write-Host "[OK] GameDVR restored"
        echo   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
        echo   Write-Host "[OK] Balanced plan restored"
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 0 -Force
        echo   Write-Host "[OK] Game mode disabled"
        echo } catch { Write-Host "[!] Error reverting configuration" }
        echo Write-Host "[OK] Reversion completed"
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    echo [*] Requesting administrator permissions...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"

:end_script
echo.
if "%action%"=="CONFIGURE" (
    if "%profile%"=="1" (
        echo [INFO] Launcher configured with basic optimizations
        echo [INFO] You can now run your launcher manually
    ) else if "%profile%"=="2" (
        echo [INFO] If UAC appears, accept to apply complete optimizations
        echo [INFO] Optimizations applied - you can run your game manually
    ) else (
        echo [INFO] Secondary process configured with balanced profile
        echo [INFO] Optimal configuration for anticheats and auxiliary processes
    )
) else (
    if "%profile%"=="4" (
        echo [INFO] Launcher configurations reverted
    ) else if "%profile%"=="5" (
        echo [INFO] If UAC appears, accept to revert complete optimizations
        echo [INFO] Game configurations reverted
    ) else (
        echo [INFO] Secondary process configurations reverted
        echo [INFO] Balanced configuration restored
    )
)

echo.
pause
exit /b