param(
    [ValidateSet('Configure', 'Revert')]
    [string] $Action,
    [ValidateSet('Launcher', 'Game')]
    [string] $Profile,
    [string] $EncodedName
)

$ErrorActionPreference = 'Stop'

function Set-SavedCpuPriority {
    param(
        [string] $ImageRoot,
        [string] $StateRoot,
        [string] $Name,
        [int] $Priority
    )

    $imagePath = Join-Path (Join-Path $ImageRoot $Name) 'PerfOptions'
    $statePath = Join-Path $StateRoot $Name
    if (-not (Test-Path -LiteralPath $statePath)) {
        New-Item -Path $statePath -Force | Out-Null
        $hadValue = 0
        if (Test-Path -LiteralPath $imagePath) {
            $image = Get-Item -LiteralPath $imagePath
            if ($image.GetValueNames() -contains 'CpuPriorityClass') {
                $hadValue = 1
                $value = $image.GetValue('CpuPriorityClass', $null,
                    [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
                New-ItemProperty -LiteralPath $statePath -Name 'OriginalValue' -Value $value `
                    -PropertyType $image.GetValueKind('CpuPriorityClass').ToString() | Out-Null
            }
        }
        New-ItemProperty -LiteralPath $statePath -Name 'HadValue' -Value $hadValue `
            -PropertyType DWord | Out-Null
    } else {
        $state = Get-Item -LiteralPath $statePath
        if ($state.GetValueNames() -notcontains 'HadValue') {
            throw "Incomplete CPU priority backup for $Name. Inspect $statePath before proceeding."
        }
    }

    if (-not (Test-Path -LiteralPath $imagePath)) {
        New-Item -Path $imagePath -Force | Out-Null
    }
    New-ItemProperty -LiteralPath $imagePath -Name 'CpuPriorityClass' -Value $Priority `
        -PropertyType DWord -Force | Out-Null
}

function Restore-SavedCpuPriority {
    param(
        [string] $ImageRoot,
        [string] $StateRoot,
        [string] $Name
    )

    $imagePath = Join-Path (Join-Path $ImageRoot $Name) 'PerfOptions'
    $statePath = Join-Path $StateRoot $Name
    if (-not (Test-Path -LiteralPath $statePath)) {
        return $false
    }

    $state = Get-Item -LiteralPath $statePath
    if ($state.GetValueNames() -notcontains 'HadValue') {
        throw "Incomplete CPU priority backup for $Name. Inspect $statePath before proceeding."
    }
    $hadValue = $state.GetValue('HadValue')
    if ($hadValue -notin @(0, 1) -or
        $state.GetValueKind('HadValue') -ne [Microsoft.Win32.RegistryValueKind]::DWord) {
        throw "Invalid CPU priority backup for $Name. Inspect $statePath before proceeding."
    }
    if ($hadValue -eq 1) {
        if ($state.GetValueNames() -notcontains 'OriginalValue') {
            throw "Original CPU priority is missing for $Name. Inspect $statePath before proceeding."
        }
        if (-not (Test-Path -LiteralPath $imagePath)) {
            New-Item -Path $imagePath -Force | Out-Null
        }
        $value = $state.GetValue('OriginalValue', $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        New-ItemProperty -LiteralPath $imagePath -Name 'CpuPriorityClass' -Value $value `
            -PropertyType $state.GetValueKind('OriginalValue').ToString() -Force | Out-Null
    } elseif (Test-Path -LiteralPath $imagePath) {
        $image = Get-Item -LiteralPath $imagePath
        if ($image.GetValueNames() -contains 'CpuPriorityClass') {
            Remove-ItemProperty -LiteralPath $imagePath -Name 'CpuPriorityClass'
        }
    }
    Remove-Item -LiteralPath $statePath
    return $true
}

if ($MyInvocation.InvocationName -eq '.') { return }

try {
    if (-not $Action -or -not $Profile) {
        throw 'Action and profile are required.'
    }

    if (-not $EncodedName) {
        $EncodedName = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($env:exename))
    }
    $name = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($EncodedName))
    if (-not $name -or $name -ne [IO.Path]::GetFileName($name) -or
        [IO.Path]::GetExtension($name) -ine '.exe' -or
        $name.IndexOfAny([IO.Path]::GetInvalidFileNameChars()) -ge 0) {
        throw 'A valid executable name ending in .exe is required.'
    }

    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        $arguments = @(
            '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ('"{0}"' -f $PSCommandPath),
            '-Action', $Action, '-Profile', $Profile, '-EncodedName', $EncodedName
        )
        $process = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -Verb RunAs `
            -ArgumentList $arguments -Wait -PassThru
        exit $process.ExitCode
    }

    $imageRoot = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options'
    $stateRoot = 'HKLM:\SOFTWARE\GameOptimizer\CpuPriorityBackup'
    if ($Action -eq 'Configure') {
        $priority = if ($Profile -eq 'Game') { 3 } else { 1 }
        Set-SavedCpuPriority -ImageRoot $imageRoot -StateRoot $stateRoot -Name $name -Priority $priority
    } else {
        if (-not (Restore-SavedCpuPriority -ImageRoot $imageRoot -StateRoot $stateRoot -Name $name)) {
            Write-Host "[WARNING] No CPU priority backup exists for $name; the current value was not changed."
            exit 2
        }
    }
    Write-Host "[OK] CPU priority: $Action - $name"
    exit 0
} catch {
    [Console]::Error.WriteLine("[ERROR] CPU priority: $($_.Exception.Message)")
    exit 1
}
