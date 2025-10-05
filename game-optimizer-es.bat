@echo off
setlocal EnableDelayedExpansion
title Optimizador de Juegos y Lanzadores

:: =====================================================
:: SISTEMA DE LOGS
:: =====================================================
:: Crear timestamp sin caracteres problematicos
for /f "tokens=1-3 delims=/ " %%a in ("%date%") do set "mydate=%%c%%b%%a"
for /f "tokens=1-3 delims=:." %%a in ("%time%") do set "mytime=%%a%%b%%c"
set "mytime=%mytime: =0%"
set "logfile=%~dp0logs\game-optimizer-es_%mydate%_%mytime%.log"
if not exist "%~dp0logs" mkdir "%~dp0logs"

:: Inicializar archivo de log
echo [%date% %time%] ========================================== > "%logfile%"
echo [%date% %time%] OPTIMIZADOR DE JUEGOS Y LANZADORES - INICIO LOG >> "%logfile%"
echo [%date% %time%] Version: 1.1.0 >> "%logfile%"
echo [%date% %time%] SO: %OS% >> "%logfile%"
echo [%date% %time%] Usuario: %USERNAME% >> "%logfile%"
echo [%date% %time%] Computadora: %COMPUTERNAME% >> "%logfile%"
echo [%date% %time%] ========================================== >> "%logfile%"

goto :main

:: =====================================================
:: FUNCIONES DE LOGGING
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
echo  OPTIMIZADOR DE JUEGOS Y LANZADORES
echo ==============================================
echo.
echo Autor: Ele
echo GitHub: https://github.com/eleazarcelis
echo Version: 1.1.0
echo ==============================================
echo.
echo Este script optimiza aplicaciones aplicando configuraciones especificas
echo segun el tipo de proceso. Los cambios son PERMANENTES hasta revertirlos.
echo.
echo ==============================================
echo  EXPLICACION DE PERFILES
echo ==============================================
echo.
echo [1] LANZADOR - Optimizacion basica para plataformas de juegos:
echo     ^* GPU: Configuracion balanceada (ahorra energia)
echo     ^* GameBar: Activado (permite overlays y capturas)
echo     ^* Plan energia: Balanceado (eficiencia energetica)
echo     ^* IMPACTO: Mejora fluidez sin consumir recursos extra
echo.
echo [2] JUEGO - Optimizacion maxima para rendimiento:
echo     ^* GPU: Alto rendimiento (usa GPU dedicada siempre)
echo     ^* Windows Defender: Excluye carpeta y ejecutable del escaneo
echo     ^* GameDVR: Desactivado (elimina lag de grabacion)
echo     ^* GameBar: Activado para modo juego automatico
echo     ^* Plan energia: Alto rendimiento (CPU y GPU al maximo)
echo     ^* IMPACTO: Maximos FPS, menor latencia, sin interrupciones
echo.
echo [3] PROCESO SECUNDARIO - Configuracion para anticheats/auxiliares:
echo     ^* GPU: Configuracion balanceada (no interfiere con juego)
echo     ^* GameBar: Desactivado (evita conflictos con anticheats)
echo     ^* Plan energia: Balanceado (no compite por recursos)
echo     ^* IMPACTO: Estabilidad maxima, sin falsos positivos
echo.
echo [4-6] OPCIONES REVERTIR: Eliminan TODAS las configuraciones aplicadas
echo       y restauran valores por defecto del sistema.
echo.
echo ==============================================
echo  SELECCIONA UNA OPCION
echo ==============================================
echo.
echo CONFIGURAR:
echo [1] LANZADOR (Steam, Epic, Origin, etc.)
echo [2] JUEGO (Optimizacion completa)
echo [3] PROCESO SECUNDARIO (Anticheats, etc.)
echo.
echo REVERTIR:
echo [4] REVERTIR LANZADOR
echo [5] REVERTIR JUEGO
echo [6] REVERTIR PROCESO SECUNDARIO
echo.

set /p "perfil=Selecciona (1, 2, 3, 4, 5 o 6): "

call :log_info "Usuario selecciono opcion: %perfil%"

if "%perfil%"=="1" (
    set "tipo=LANZADOR"
    set "accion=CONFIGURAR"
) else if "%perfil%"=="2" (
    set "tipo=JUEGO"
    set "accion=CONFIGURAR"
) else if "%perfil%"=="3" (
    set "tipo=PROCESO SECUNDARIO"
    set "accion=CONFIGURAR"
) else if "%perfil%"=="4" (
    set "tipo=LANZADOR"
    set "accion=REVERTIR"
) else if "%perfil%"=="5" (
    set "tipo=JUEGO"
    set "accion=REVERTIR"
) else if "%perfil%"=="6" (
    set "tipo=PROCESO SECUNDARIO"
    set "accion=REVERTIR"
) else (
    call :log_error "Opcion invalida seleccionada: %perfil%"
    echo Error: Opcion invalida
    pause
    exit /b 1
)

echo.
echo ==============================================
echo  %accion% %tipo%
echo ==============================================
echo.

call :log_info "Iniciando %accion% para %tipo%"

set /p "exe=Ruta completa del ejecutable: "
set "exe=%exe:"=%"

call :log_debug "Ruta de ejecutable ingresada: %exe%"

if "%exe%"=="" (
    call :log_error "No se proporciono ruta de ejecutable"
    echo Error: No se ingreso ruta
    pause
    exit /b 1
)

if not exist "%exe%" (
    call :log_error "Archivo ejecutable no encontrado: %exe%"
    echo Error: Archivo no encontrado
    pause  
    exit /b 1
)

for %%I in ("%exe%") do (
    set "folder=%%~dpI"
    set "exename=%%~nxI"
)
if "%folder:~-1%"=="\" set "folder=%folder:~0,-1%"

call :log_info "Procesamiento de archivo completado exitosamente"
call :log_debug "Ejecutable: %exe%"
call :log_debug "Carpeta: %folder%"
call :log_debug "Nombre: %exename%"

echo.
echo Ejecutable: "%exe%"
echo Carpeta: "%folder%"
echo Nombre: "%exename%"
echo.

:: =====================================================
:: Configurar/Revertir GPU
:: =====================================================
if "%accion%"=="CONFIGURAR" (
    call :log_info "Iniciando configuracion GPU para %tipo%"
    echo [*] Configurando preferencia GPU...
    
    if "%perfil%"=="1" (
        call :log_debug "Configurando preferencia GPU balanceada para lanzador"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] GPU Balanceada configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Fallo al configurar preferencia GPU balanceada"
    ) else if "%perfil%"=="2" (
        call :log_debug "Configurando preferencia GPU alto rendimiento para juego"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=2;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 2 -Force; Write-Host '[OK] GPU Alto Rendimiento configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Fallo al configurar preferencia GPU alto rendimiento"
    ) else (
        call :log_debug "Configurando preferencia GPU balanceada para proceso secundario"
        powershell -NoProfile -Command "try { $exePath = '%exe%'; if (-not (Test-Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences')) { New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null }; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] GPU Balanceada (Proceso Secundario) configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
        if errorlevel 1 call :log_error "Fallo al configurar preferencia GPU balanceada para proceso secundario"
    )
) else (
    call :log_info "Iniciando reversion de preferencias GPU"
    echo [*] Revirtiendo preferencia GPU...
    
    powershell -NoProfile -Command "try { $exePath = '%exe%'; Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Force -ErrorAction SilentlyContinue; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; Remove-Item -Path $regPath -Recurse -Force -ErrorAction SilentlyContinue; Write-Host '[OK] Configuracion GPU eliminada' } catch { Write-Host '[!] Error al revertir GPU' }"
    if errorlevel 1 call :log_error "Fallo al revertir preferencias GPU"
)

:: Aplicar/Revertir Optimizaciones
echo.
if "%accion%"=="CONFIGURAR" (
    call :log_info "Iniciando configuracion de optimizaciones"
    echo [*] Aplicando optimizaciones...
    
    if "%perfil%"=="1" (
        :: LAUNCHER PROFILE - CONFIGURAR
        call :log_debug "Aplicando optimizaciones para lanzador"
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        if errorlevel 1 call :log_error "Fallo al habilitar modo automatico GameBar"
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        if errorlevel 1 call :log_error "Fallo al establecer plan de energia balanceado"
        call :log_info "Configuracion de lanzador completada exitosamente"
        echo [OK] Configuracion basica aplicada
    ) else if "%perfil%"=="2" (
        :: GAME PROFILE - CONFIGURAR
        goto :configure_game
    ) else (
        :: SECONDARY PROCESS PROFILE - CONFIGURAR
        call :log_debug "Aplicando optimizaciones para proceso secundario"
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        if errorlevel 1 call :log_error "Fallo al deshabilitar modo automatico GameBar"
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        if errorlevel 1 call :log_error "Fallo al establecer plan de energia balanceado"
        call :log_info "Configuracion de proceso secundario completada exitosamente"
        echo [OK] Configuracion proceso secundario aplicada
    )
) else (
    call :log_info "Iniciando reversion de optimizaciones"
    echo [*] Revirtiendo optimizaciones...
    
    if "%perfil%"=="4" (
        :: LAUNCHER PROFILE - REVERTIR
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Configuracion lanzador revertida
    ) else if "%perfil%"=="5" (
        :: GAME PROFILE - REVERTIR
        goto :revert_game
    ) else (
        :: SECONDARY PROCESS PROFILE - REVERTIR
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Configuracion proceso secundario revertida
    )
)
goto :end_script

:configure_game
    :: GAME PROFILE
    call :log_info "Iniciando optimizaciones avanzadas para perfil de juego"
    echo [*] Creando optimizaciones avanzadas...
    
    set "ps1=%temp%\gameopt.ps1"
    call :log_debug "Creando script PowerShell: %ps1%"
    
    (
        echo Write-Host "[*] Optimizando para juego..."
        echo $logFile = "%logfile:~0,-4%_powershell.log"
        echo "[{0}] [INFO] Iniciando script PowerShell de optimizacion de juego" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo try {
        echo   Add-MpPreference -ExclusionPath "!folder!"
        echo   Add-MpPreference -ExclusionProcess "!exe!"
        echo   Write-Host "[OK] Exclusiones Defender"
        echo   "[{0}] [INFO] Exclusiones de Windows Defender agregadas exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error Defender" 
        echo   "[{0}] [ERROR] Fallo al agregar exclusiones de Windows Defender: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 0 -Force
        echo   Write-Host "[OK] GameDVR desactivado"
        echo   "[{0}] [INFO] GameDVR desactivado exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
        echo   Write-Host "[OK] Alto rendimiento"
        echo   "[{0}] [INFO] Plan de energia alto rendimiento activado" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 1 -Force
        echo   Write-Host "[OK] Modo juego activado"
        echo   "[{0}] [INFO] Modo juego activado exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error configuracion" 
        echo   "[{0}] [ERROR] Error de configuracion de juego: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo "[{0}] [INFO] Script PowerShell de optimizacion de juego completado" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    call :log_info "Solicitando permisos de administrador para optimizaciones de juego"
    echo [*] Solicitando permisos administrador...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    if errorlevel 1 call :log_error "Fallo al lanzar script PowerShell elevado"
    goto :end_script

:revert_game
    call :log_info "Iniciando reversion de optimizaciones de juego"
    echo [*] Creando script de reversion...
    
    set "ps1=%temp%\gamerevert.ps1"
    call :log_debug "Creando script PowerShell de reversion: %ps1%"
    
    (
        echo Write-Host "[*] Revirtiendo optimizaciones de juego..."
        echo $logFile = "%logfile:~0,-4%_powershell.log"
        echo "[{0}] [INFO] Iniciando script PowerShell de reversion de juego" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo try {
        echo   Remove-MpPreference -ExclusionPath "!folder!" -ErrorAction SilentlyContinue
        echo   Remove-MpPreference -ExclusionProcess "!exe!" -ErrorAction SilentlyContinue
        echo   Write-Host "[OK] Exclusiones Defender eliminadas"
        echo   "[{0}] [INFO] Exclusiones de Windows Defender eliminadas exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error al revertir Defender" 
        echo   "[{0}] [ERROR] Fallo al eliminar exclusiones de Windows Defender: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 1 -Force
        echo   Write-Host "[OK] GameDVR restaurado"
        echo   "[{0}] [INFO] GameDVR restaurado exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
        echo   Write-Host "[OK] Plan balanceado restaurado"
        echo   "[{0}] [INFO] Plan de energia balanceado restaurado" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 0 -Force
        echo   Write-Host "[OK] Modo juego desactivado"
        echo   "[{0}] [INFO] Modo juego desactivado exitosamente" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo } catch { 
        echo   Write-Host "[!] Error al revertir configuracion" 
        echo   "[{0}] [ERROR] Error de reversion de juego: {1}" -f ^(Get-Date^), $_.Exception.Message ^| Out-File -FilePath $logFile -Append
        echo }
        echo Write-Host "[OK] Reversion completada"
        echo "[{0}] [INFO] Script PowerShell de reversion de juego completado" -f ^(Get-Date^) ^| Out-File -FilePath $logFile -Append
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    call :log_info "Solicitando permisos de administrador para reversion de juego"
    echo [*] Solicitando permisos administrador...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    if errorlevel 1 call :log_error "Fallo al lanzar script PowerShell de reversion elevado"

:end_script
call :log_info "Ejecucion del script completada - %accion% %tipo%"
echo.
if "%accion%"=="CONFIGURAR" (
    if "%perfil%"=="1" (
        echo [INFO] Lanzador configurado con optimizaciones basicas
        echo [INFO] Ahora puedes ejecutar tu lanzador manualmente
    ) else if "%perfil%"=="2" (
        echo [INFO] Si aparece UAC, acepta para aplicar optimizaciones completas
        echo [INFO] Optimizaciones aplicadas - puedes ejecutar tu juego manualmente
    ) else (
        echo [INFO] Proceso secundario configurado con perfil balanceado
        echo [INFO] Configuracion optima para anticheats y procesos auxiliares
    )
) else (
    if "%perfil%"=="4" (
        echo [INFO] Configuraciones del lanzador revertidas
    ) else if "%perfil%"=="5" (
        echo [INFO] Si aparece UAC, acepta para revertir optimizaciones completas
        echo [INFO] Configuraciones del juego revertidas
    ) else (
        echo [INFO] Configuraciones del proceso secundario revertidas
        echo [INFO] Configuracion balanceada restaurada
    )
)

echo.
echo Archivo de log creado en: %logfile%
call :log_info "=========================================="
call :log_info "OPTIMIZADOR DE JUEGOS Y LANZADORES - FIN LOG"
call :log_info "=========================================="
pause
exit /b
