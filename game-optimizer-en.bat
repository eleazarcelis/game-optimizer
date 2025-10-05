@echo off
setlocal EnableDelayedExpansion
title Game and Launcher Optimizer

:: =====================================================
:: LOGGING SYSTEM
:: =====================================================
:: Create timestamp without problematic characters
for /f "tokens=1-3 delims=/ " %%a in ("%date%") do set "mydate=%%c%%b%%a"
for /f "tokens=1-3 delims=:." %%a in ("%time%") do set "mytime=%%a%%b%%c"
set "mytime=%mytime: =0%"
set "logfile=%~dp0logs\game-optimizer_%mydate%_%mytime%.log"
if not exist "%~dp0logs" mkdir "%~dp0logs"

:: Initialize log file
echo [%date% %time%] ========================================== > "%logfile%"
echo [%date% %time%] GAME AND LAUNCHER OPTIMIZER - LOG START >> "%logfile%"
echo [%date% %time%] Version: 1.1.0 >> "%logfile%"
echo [%date% %time%] OS: %OS% >> "%logfile%"
echo [%date% %time%] User: %USERNAME% >> "%logfile%"
echo [%date% %time%] Computer: %COMPUTERNAME% >> "%logfile%"
echo [%date% %time%] ========================================== >> "%logfile%"

goto :main

:: =====================================================
:: LOGGING FUNCTIONS
:: =====================================================
:log_info
    echo [%date% %time%] [INFO] %~1 >> "%logfile%"
    echo [INFO] %~1
    goto :eof

:log_error
    echo [%date% %time%] [ERROR] %~1 >> "%logfile%"
    echo [ERROR] %~1
    goto :eof

:log_debug
    echo [%date% %time%] [DEBUG] %~1 >> "%logfile%"
    echo [DEBUG] %~1
    goto :eof

:log_warning
    echo [%date% %time%] [WARNING] %~1 >> "%logfile%"
    echo [WARNING] %~1
    goto :eof

:main
echo ==============================================
echo  GAME AND LAUNCHER OPTIMIZER
echo ==============================================
echo.
echo Author: Ele
echo GitHub: https://github.com/eleazarcelis
echo Version: 1.1.0
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

call :log_info "User selected option: %profile%"

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
    call :log_error "Invalid option selected: %profile%"
    echo Error: Invalid option
    pause
    exit /b 1
)

echo.
echo ==============================================
echo  %action% %type%
echo ==============================================
echo.

call :log_info "Starting %action% for %type%"

set /p "exe=Full path to executable: "
set "exe=%exe:"=%"

call :log_debug "Executable path entered: %exe%"

if "%exe%"=="" (
    call :log_error "No executable path provided by user"
    echo Error: No path entered
    pause
    exit /b 1
)

if not exist "%exe%" (
    call :log_error "Executable file not found: %exe%"
    echo Error: File not found
    pause  
    exit /b 1
)

for %%I in ("%exe%") do (
    set "folder=%%~dpI"
    set "exename=%%~nxI"
)
if "%folder:~-1%"=="\" set "folder=%folder:~0,-1%"

call :log_info "File processing completed successfully"
call :log_debug "Executable: %exe%"
call :log_debug "Folder: %folder%"
call :log_debug "Name: %exename%"

echo.
echo Executable: "%exe%"
echo Folder: "%folder%"
echo Name: "%exename%"
echo.

:: =====================================================
:: Configure/Revert GPU
:: =====================================================
if "%action%"=="CONFIGURE" (
    call :log_info "Starting GPU configuration for %type%"
    echo [*] Configuring GPU preference...
    
    if "%profile%"=="1" (
        call :log_debug "Configuring balanced GPU preference for launcher"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] Balanced GPU configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Failed to configure balanced GPU preference"
    ) else if "%profile%"=="2" (
        call :log_debug "Configuring high performance GPU preference for game"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=2;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 2 -Force; Write-Host '[OK] High Performance GPU configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Failed to configure high performance GPU preference"
    ) else (
        call :log_debug "Configuring balanced GPU preference for secondary process"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] Balanced GPU (Secondary Process) configured' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Failed to configure balanced GPU preference for secondary process"
    )
) else (
    call :log_info "Starting GPU preference reversion"
    echo [*] Reverting GPU preference...
    
    powershell -NoProfile -Command "try { $exePath = '%exe%'; Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Force -ErrorAction SilentlyContinue; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; Remove-Item -Path $regPath -Recurse -Force -ErrorAction SilentlyContinue; Write-Host '[OK] GPU configuration removed' } catch { Write-Host '[!] Error reverting GPU' }"
    if errorlevel 1 call :log_error "Failed to revert GPU preferences"
)

:: Apply/Revert Optimizations
echo.
if "%action%"=="CONFIGURE" (
    call :log_info "Starting optimization configuration"
    echo [*] Applying optimizations...
    
    if "%profile%"=="1" (
        :: LAUNCHER PROFILE - CONFIGURE
        call :log_debug "Applying launcher optimizations"
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        if errorlevel 1 call :log_error "Failed to enable GameBar auto mode"
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        if errorlevel 1 call :log_error "Failed to set balanced power plan"
        call :log_info "Launcher configuration completed successfully"
        echo [OK] Basic configuration applied
    ) else if "%profile%"=="2" (
        :: GAME PROFILE - CONFIGURE
        goto :configure_game
    ) else (
        :: SECONDARY PROCESS PROFILE - CONFIGURE
        call :log_debug "Applying secondary process optimizations"
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        if errorlevel 1 call :log_error "Failed to disable GameBar auto mode"
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        if errorlevel 1 call :log_error "Failed to set balanced power plan"
        call :log_info "Secondary process configuration completed successfully"
        echo [OK] Secondary process configuration applied
    )
) else (
    call :log_info "Starting optimization reversion"
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
    call :log_info "Starting game profile advanced optimizations"
    echo [*] Creating advanced optimizations...
    
    set "ps1=%temp%\gameopt.ps1"
    call :log_debug "Creating PowerShell script: %ps1%"
    
    (
        echo Write-Host "[*] Optimizing for game..."
        echo $logFile = "%logfile:~0,-4%_powershell.log"
        echo "[{0}] [INFO] Starting PowerShell game optimization script" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo try {
        echo   Add-MpPreference -ExclusionPath "!folder!"
        echo   Add-MpPreference -ExclusionProcess "!exe!"
        echo   Write-Host "[OK] Defender exclusions"
        echo   "[{0}] [INFO] Windows Defender exclusions added successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Defender error" 
        echo   "[{0}] [ERROR] Failed to add Windows Defender exclusions: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 0 -Force
        echo   Write-Host "[OK] GameDVR disabled"
        echo   "[{0}] [INFO] GameDVR disabled successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
        echo   Write-Host "[OK] High performance"
        echo   "[{0}] [INFO] High performance power plan activated" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 1 -Force
        echo   Write-Host "[OK] Game mode activated"
        echo   "[{0}] [INFO] Game mode activated successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Configuration error" 
        echo   "[{0}] [ERROR] Game configuration error: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo "[{0}] [INFO] PowerShell game optimization script completed" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    call :log_info "Requesting administrator permissions for game optimizations"
    echo [*] Requesting administrator permissions...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    if errorlevel 1 call :log_error "Failed to launch elevated PowerShell script"
    goto :end_script

:revert_game
    call :log_info "Starting game optimization reversion"
    echo [*] Creating reversion script...
    
    set "ps1=%temp%\gamerevert.ps1"
    call :log_debug "Creating reversion PowerShell script: %ps1%"
    
    (
        echo Write-Host "[*] Reverting game optimizations..."
        echo $logFile = "%logfile:~0,-4%_powershell.log"
        echo "[{0}] [INFO] Starting PowerShell game reversion script" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo try {
        echo   Remove-MpPreference -ExclusionPath "!folder!" -ErrorAction SilentlyContinue
        echo   Remove-MpPreference -ExclusionProcess "!exe!" -ErrorAction SilentlyContinue
        echo   Write-Host "[OK] Defender exclusions removed"
        echo   "[{0}] [INFO] Windows Defender exclusions removed successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error reverting Defender" 
        echo   "[{0}] [ERROR] Failed to remove Windows Defender exclusions: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 1 -Force
        echo   Write-Host "[OK] GameDVR restored"
        echo   "[{0}] [INFO] GameDVR restored successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
        echo   Write-Host "[OK] Balanced plan restored"
        echo   "[{0}] [INFO] Balanced power plan restored" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 0 -Force
        echo   Write-Host "[OK] Game mode disabled"
        echo   "[{0}] [INFO] Game mode disabled successfully" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error reverting configuration" 
        echo   "[{0}] [ERROR] Game reversion error: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo Write-Host "[OK] Reversion completed"
        echo "[{0}] [INFO] PowerShell game reversion script completed" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    call :log_info "Requesting administrator permissions for game reversion"
    echo [*] Requesting administrator permissions...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    if errorlevel 1 call :log_error "Failed to launch elevated PowerShell reversion script"

:end_script
call :log_info "Script execution completed - %action% %type%"
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
echo Log file created at: %logfile%
call :log_info "=========================================="
call :log_info "GAME AND LAUNCHER OPTIMIZER - LOG END"
call :log_info "=========================================="
pause
exit /b