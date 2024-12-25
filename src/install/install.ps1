if (!$IsWindows) {
    Write-Output "Sorry, but this powershell script currently doesn't support powershell core."
    Exit
}

$workdir = "$($Env:temp)\tmp$([convert]::tostring((get-random 65535),16).padleft(4,'0')).tmp"
New-Item -ItemType Directory -Path $workdir

function cleanup {
    Pop-Location
    Remove-Item $workdir -Recurse -Force
}

trap {
    cleanup
}

$PARENT_DIR = ""

if ( $VERSION -ge 3 ) {
    $PARENT_DIR = $PSScriptRoot
}
else {
    $PARENT_DIR = split-path -parent $MyInvocation.MyCommand.Definition
}

$FORCE_TAR = $env:FORCE_TAR ?? $false
$USE_TAR = $true

if (-not (Get-Command tar -ErrorAction Ignore)) {
    if ($FORCE_TAR) {
        Write-Output "FORCE_TAR is set and tar is uninstalled, please either update your system or unset FORCE_TAR!"
        cleanup
        Exit
    }
    Write-Output "Cannot find tar, using Expand-7Zip"
    if (-not (Get-Command Expand-7Zip -ErrorAction Ignore)) {
        Write-Output "Importing bundled version of 7Zip4Powershell(could not find system version)..."
        $pathToModule = "$PARENT_DIR\7Zip4Powershell\2.7.0\7Zip4PowerShell.psd1"
        Import-Module $pathToModule
        $USE_TAR = $false
    }
}

$FORMAT = $env:FORMAT ?? ".tar.gz"

if ($USE_TAR) {
    if (!(tar --help | Select-String -Pattern gzip)) {
        if (!(tar --help | Select-String -Pattern xz)) {
            Write-Output "Tar does not support gzip or xz, using Expand-7Zip"
            if (-not (Get-Command Expand-7Zip -ErrorAction Ignore)) {
                Write-Output "Importing bundled version of 7Zip4Powershell(could not find system version)..."
                $pathToModule = "$PARENT_DIR\7Zip4Powershell\2.7.0\7Zip4PowerShell.psd1"
                Import-Module $pathToModule
                $USE_TAR = $false
            }
        }
        else {
            if ($FORMAT -eq ".tar.gz") {
                Write-Output "Tar does not support gzip, using xz"
                $FORMAT = ".tar.xz"
            }
        }
    }
    else {
        if (!(tar --help | Select-String -Pattern xz)) {
            if ($FORMAT -eq ".tar.xz") {
                Write-Output "Tar does not support xz, using gzip"
                $FORMAT = ".tar.gz"
            }
        }
    }
}

Push-Location .

$USING_MSI = $env:USING_MSI ?? &?TOOLCHAIN.MSI
$VERSION = $PSVersionTable.PSVersion.Major ?? 1

$COMPONENTS = $env:COMPONENTS ?? "&?TOOLCHAIN.COMPONENTS"
$CHANNEL = $env:CHANNEL ?? "&?TOOLCHAIN.CHANNEL"

$TAR_FLAGS = $env:TAR_FLAGS ?? "-xf"
$EXPAND_7ZIP_FLAGS = $env:EXPAND_7ZIP_FLAGS ?? ""
$MSI_FLAGS = $env:MSI_FLAGS ?? ""

Set-Location $PARENT_DIR

if ($USING_MSI) {
    Start-Process msiexec "$MSI_FLAGS $PARENT_DIR\toolchain\rust-$CHANNEL-$target_triple.msi";
}
$arch = "i686"
if ([System.Environment]::Is64BitProcess) {
    $arch = "x86_64"
}

$target_triple = "$arch-pc-windows-msvc"

$COMPONENTS.Split(" ") | ForEach-Object {
    $comp = $_
    if (! $(Test-Path "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT" -PathType Leaf)) {
        if ($env:IGNORE_NONEXISTANT_COMPONENTS) {
            return # actually continue, but foreach-object is weird
        }
        Write-Output "Expected file $PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT to exist, but doesn't!"
        Write-Output "(hint: to ignore and continue, set the environment variable IGNORE_NONEXISTANT_COMPONENTS to any truthy value)"
        cleanup
        Exit
    }
    if (! $env:DONT_HASH) {
        if (! $(Test-Path "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT.sha256" -PathType Leaf)) {
            Write-Output "Expected file $PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT.sha256 to exist, but doesn't!"
            Write-Output "(hint: to ignore and continue, set the environment variable DONT_HASH to any truthy value)"
            cleanup
            Exit
        }
        $HASH1 = $(Get-FileHash "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT" -Algorithm SHA256)
        $HASH2 = $(Get-Content "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT.sha256")
        if ($HASH1 -ne $HASH2) {
            Write-Output "Error: Hash of $PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT does not match expected hash!"
            Write-Output "(hint: to disable this warning, set the environment variable DONT_HASH to 1)"
            cleanup
            Exit
        }
        Write-Output "Tested - hash and expected hash of $comp-$CHANNEL-$target_triple.$FORMAT match($HASH1)!"
    }
    New-Item -Path $(Get-Location) -Name "$comp-$CHANNEL-$target_triple" -ItemType "directory"
    Set-Location "$comp-$CHANNEL-$target_triple"
    if ($USE_TAR) {
        tar $TAR_FLAGS "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT"
    }
    else {
        Expand-7Zip "$PARENT_DIR\toolchain\$comp-$CHANNEL-$target_triple.$FORMAT" $(Get-Location) $EXPAND_7ZIP_FLAGS
    }
    Set-Location "$comp-$CHANNEL-$target_triple"
    New-Item -Path "$env:USERPROFILE" -Name ".cargo" -ItemType Directory
    New-Item -Path "$env:USERPROFILE\.cargo" -Name "bin" -ItemType Directory
    (Get-Content "$(Get-Location)\components").Split("`n") | ForEach-Object {
        Copy-Item "$(Get-Location)\$_\bin\*" "$env:USERPROFILE\.cargo\bin" -Recurse
    }
}

cleanup