@echo off
setlocal EnableDelayedExpansion
title Optimizador de Juegos y Lanzadores

echo ==============================================
echo  OPTIMIZADOR DE JUEGOS Y LANZADORES
echo ==============================================
echo.
echo Autor: Ele
echo GitHub: https://github.com/eleazarcelis
echo Version: 1.0.0
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
    echo Error: Opcion invalida
    pause
    exit /b 1
)

echo.
echo ==============================================
echo  %accion% %tipo%
echo ==============================================
echo.

set /p "exe=Ruta completa del ejecutable: "
set "exe=%exe:"=%"

if "%exe%"=="" (
    echo Error: No se ingreso ruta
    pause
    exit /b 1
)

if not exist "%exe%" (
    echo Error: Archivo no encontrado
    pause  
    exit /b 1
)

for %%I in ("%exe%") do (
    set "folder=%%~dpI"
    set "exename=%%~nxI"
)
if "%folder:~-1%"=="\" set "folder=%folder:~0,-1%"

echo.
echo Ejecutable: "%exe%"
echo Carpeta: "%folder%"
echo Nombre: "%exename%"
echo.

:: =====================================================
:: Configurar/Revertir GPU
:: =====================================================
if "%accion%"=="CONFIGURAR" (
    echo [*] Configurando preferencia GPU...
    
    if "%perfil%"=="1" (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] GPU Balanceada configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    ) else if "%perfil%"=="2" (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=2;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 2 -Force; Write-Host '[OK] GPU Alto Rendimiento configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    ) else (
        powershell -NoProfile -Command "try { $exePath = '%exe%'; New-Item -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Force | Out-Null; Set-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Value 'GpuPreference=1;' -Force; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; New-Item -Path $regPath -Force | Out-Null; Set-ItemProperty -Path $regPath -Name 'MatchedExeFullPath' -Value $exePath -Force; Set-ItemProperty -Path $regPath -Name 'GpuPreference' -Value 1 -Force; Write-Host '[OK] GPU Balanceada (Proceso Secundario) configurada' } catch { Write-Host '[!] GPU Error:' $_.Exception.Message }"
    )
) else (
    echo [*] Revirtiendo preferencia GPU...
    
    powershell -NoProfile -Command "try { $exePath = '%exe%'; Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\DirectX\UserGpuPreferences' -Name $exePath -Force -ErrorAction SilentlyContinue; $hash = [System.Security.Cryptography.MD5]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($exePath)); $hashString = [System.BitConverter]::ToString($hash) -replace '-'; $regPath = \"HKCU:\System\GameConfigStore\Children\$hashString\"; Remove-Item -Path $regPath -Recurse -Force -ErrorAction SilentlyContinue; Write-Host '[OK] Configuracion GPU eliminada' } catch { Write-Host '[!] Error al revertir GPU' }"
)

:: Aplicar/Revertir Optimizaciones
echo.
if "%accion%"=="CONFIGURAR" (
    echo [*] Aplicando optimizaciones...
    
    if "%perfil%"=="1" (
        :: LAUNCHER PROFILE - CONFIGURAR
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 1 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Configuracion basica aplicada
    ) else if "%perfil%"=="2" (
        :: GAME PROFILE - CONFIGURAR
        goto :configure_game
    ) else (
        :: SECONDARY PROCESS PROFILE - CONFIGURAR
        powershell -NoProfile -Command "Set-ItemProperty 'HKCU:\Software\Microsoft\GameBar' -Name 'AutoGameModeEnabled' -Value 0 -Force" 2>nul
        powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e >nul 2>&1
        
        echo [OK] Configuracion proceso secundario aplicada
    )
) else (
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
    echo [*] Creando optimizaciones avanzadas...
    
    set "ps1=%temp%\gameopt.ps1"
    
    (
        echo Write-Host "[*] Optimizando para juego..."
        echo try {
        echo   Add-MpPreference -ExclusionPath "!folder!"
        echo   Add-MpPreference -ExclusionProcess "!exe!"
        echo   Write-Host "[OK] Exclusiones Defender"
        echo } catch { Write-Host "[!] Error Defender" }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 0 -Force
        echo   Write-Host "[OK] GameDVR desactivado"
        echo   powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c
        echo   Write-Host "[OK] Alto rendimiento"
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 1 -Force
        echo   Write-Host "[OK] Modo juego activado"
        echo } catch { Write-Host "[!] Error configuracion" }

        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    echo [*] Solicitando permisos administrador...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"
    goto :end_script

:revert_game
    echo [*] Creando script de reversion...
    
    set "ps1=%temp%\gamerevert.ps1"
    
    (
        echo Write-Host "[*] Revirtiendo optimizaciones de juego..."
        echo try {
        echo   Remove-MpPreference -ExclusionPath "!folder!" -ErrorAction SilentlyContinue
        echo   Remove-MpPreference -ExclusionProcess "!exe!" -ErrorAction SilentlyContinue
        echo   Write-Host "[OK] Exclusiones Defender eliminadas"
        echo } catch { Write-Host "[!] Error al revertir Defender" }
        echo try {
        echo   Set-ItemProperty "HKCU:\System\GameConfigStore" -Name "GameDVR_Enabled" -Value 1 -Force
        echo   Write-Host "[OK] GameDVR restaurado"
        echo   powercfg /setactive 381b4222-f694-41f0-9685-ff5bb260df2e
        echo   Write-Host "[OK] Plan balanceado restaurado"
        echo   Set-ItemProperty "HKCU:\Software\Microsoft\GameBar" -Name "AutoGameModeEnabled" -Value 0 -Force
        echo   Write-Host "[OK] Modo juego desactivado"
        echo } catch { Write-Host "[!] Error al revertir configuracion" }
        echo Write-Host "[OK] Reversion completada"
        echo Remove-Item $PSCommandPath -Force
    ) > "!ps1!"
    
    echo [*] Solicitando permisos administrador...
    powershell -NoProfile -ExecutionPolicy Bypass -Command "Start-Process PowerShell -Verb RunAs -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', '!ps1!'"

:end_script
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
pause
exit /b
