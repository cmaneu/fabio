$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot '..' 'install.ps1')
$InstallerPath = Join-Path $PSScriptRoot '..' 'install.ps1'

function Assert-Equal {
    param(
        [object]$Expected,
        [object]$Actual,
        [string]$Message
    )

    if ($Expected -ne $Actual) {
        throw "$Message (expected '$Expected', got '$Actual')"
    }
}

$script:LatestRequests = 0
function Invoke-RestMethod {
    param([string]$Uri)

    $script:LatestRequests++
    Assert-Equal 'https://api.github.com/repos/iemejia/fabio/releases/latest' $Uri 'Latest release URL differs'
    return [pscustomobject]@{ tag_name = 'v9.8.7' }
}

$PreviousFabioVersion = $env:FABIO_VERSION
try {
    $env:FABIO_VERSION = '0.77.0'
    . $InstallerPath
    Assert-Equal '0.77.0' $Version 'FABIO_VERSION was not used as the default version'

    . $InstallerPath -Version '0.76.0'
    Assert-Equal '0.76.0' $Version 'The Version parameter was not bound'
} finally {
    $env:FABIO_VERSION = $PreviousFabioVersion
}

Assert-Equal 'v0.78.0' (Resolve-FabioReleaseTag '0.78.0') 'Unprefixed version was not normalized'
Assert-Equal 'v0.78.0' (Resolve-FabioReleaseTag 'v0.78.0') 'Prefixed version changed'
Assert-Equal 'v0.79.0-rc.1' (Resolve-FabioReleaseTag '0.79.0-rc.1') 'Prerelease version was not normalized'
Assert-Equal 0 $script:LatestRequests 'A requested version queried the latest release API'

Assert-Equal 'v9.8.7' (Resolve-FabioReleaseTag '') 'Empty version did not resolve latest release'
Assert-Equal 1 $script:LatestRequests 'Latest release API request count differs'

try {
    Resolve-FabioReleaseTag '../invalid' | Out-Null
    throw 'Invalid version was accepted'
} catch {
    if ($_.Exception.Message -notlike "Invalid Fabio version*") {
        throw
    }
}

Write-Host 'install.ps1 tests passed'
