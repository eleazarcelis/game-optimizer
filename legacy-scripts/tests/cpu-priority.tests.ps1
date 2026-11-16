$ErrorActionPreference = 'Stop'
. "$PSScriptRoot\..\cpu-priority.ps1"

function Assert-Equal($Actual, $Expected, $Message) {
    if ($Actual -cne $Expected) {
        throw "${Message}: expected '$Expected', got '$Actual'"
    }
}

$root = 'HKCU:\Software\GameOptimizerTest-' + [guid]::NewGuid().ToString('N')
$images = Join-Path $root 'Images'
$backups = Join-Path $root 'Backups'
$name = 'example.exe'
$options = Join-Path (Join-Path $images $name) 'PerfOptions'

try {
    New-Item -Path $options -Force | Out-Null
    New-ItemProperty -LiteralPath $options -Name 'OtherSetting' -Value 42 -PropertyType DWord | Out-Null

    Set-SavedCpuPriority $images $backups $name 1
    Assert-Equal (Get-ItemPropertyValue -LiteralPath $options -Name 'CpuPriorityClass') 1 'Launcher priority'
    Set-SavedCpuPriority $images $backups $name 3
    Assert-Equal (Get-ItemPropertyValue -LiteralPath $options -Name 'CpuPriorityClass') 3 'Game priority'
    Assert-Equal (Get-ItemPropertyValue -LiteralPath $options -Name 'OtherSetting') 42 'Unrelated setting'
    Assert-Equal (Restore-SavedCpuPriority $images $backups $name) $true 'Restore without previous value'
    $key = Get-Item -LiteralPath $options
    Assert-Equal ($key.GetValueNames() -contains 'CpuPriorityClass') $false 'Original absence restored'
    Assert-Equal ($key.GetValue('OtherSetting')) 42 'Unrelated setting after restore'
    Assert-Equal (Restore-SavedCpuPriority $images $backups $name) $false 'Missing backup'

    New-ItemProperty -LiteralPath $options -Name 'CpuPriorityClass' -Value 5 -PropertyType DWord | Out-Null
    Set-SavedCpuPriority $images $backups $name 3
    Assert-Equal (Restore-SavedCpuPriority $images $backups $name) $true 'Restore existing value'
    $key = Get-Item -LiteralPath $options
    Assert-Equal ($key.GetValue('CpuPriorityClass')) 5 'Original priority restored'
    Assert-Equal ($key.GetValueKind('CpuPriorityClass')) ([Microsoft.Win32.RegistryValueKind]::DWord) 'Original value type'

    New-ItemProperty -LiteralPath $options -Name 'CpuPriorityClass' -Value 'previous' -PropertyType String -Force | Out-Null
    Set-SavedCpuPriority $images $backups $name 1
    Assert-Equal (Restore-SavedCpuPriority $images $backups $name) $true 'Restore non-DWORD value'
    $key = Get-Item -LiteralPath $options
    Assert-Equal ($key.GetValue('CpuPriorityClass')) 'previous' 'Original non-DWORD value'
    Assert-Equal ($key.GetValueKind('CpuPriorityClass')) ([Microsoft.Win32.RegistryValueKind]::String) 'Original non-DWORD type'

    $specialName = 'game[1] & more.exe'
    $specialOptions = Join-Path (Join-Path $images $specialName) 'PerfOptions'
    Set-SavedCpuPriority $images $backups $specialName 3
    Assert-Equal (Get-ItemPropertyValue -LiteralPath $specialOptions -Name 'CpuPriorityClass') 3 'Special characters'
    Assert-Equal (Restore-SavedCpuPriority $images $backups $specialName) $true 'Restore special-character name'

    Write-Host 'CPU priority tests passed.'
} finally {
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
}
