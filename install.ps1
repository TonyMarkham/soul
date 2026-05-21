[CmdletBinding()]
param(
    [string]$Version = "latest",
    [string]$Archive,
    [string]$Target = ".",
    [string]$ConfigPath = (Join-Path $HOME ".config\opencode\opencode.json"),
    [string]$Repo = "TonyMarkham/soul",
    [switch]$SkipOpencodeConfig
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-FullPath {
    param([string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }

    return [System.IO.Path]::GetFullPath((Join-Path (Get-Location).Path $Path))
}

$asset = switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { "soul-windows-x64.zip"; break }
    default { throw "Unsupported Windows architecture: $env:PROCESSOR_ARCHITECTURE" }
}

if ($Version -eq "latest") {
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
} else {
    $url = "https://github.com/$Repo/releases/download/$Version/$asset"
}

$targetRoot = Resolve-FullPath $Target
$targetSoulDir = Join-Path $targetRoot ".soul"
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
$archivePath = Join-Path $tempRoot $asset
$extractPath = Join-Path $tempRoot "extract"

New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
New-Item -ItemType Directory -Path $extractPath -Force | Out-Null

try {
    if ($Archive) {
        $archivePath = Resolve-FullPath $Archive
        if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
            throw "Archive not found: $archivePath"
        }
    } else {
        Invoke-WebRequest -Uri $url -OutFile $archivePath
    }

    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractPath -Force

    $sourceSoulDir = Join-Path $extractPath ".soul"
    if (-not (Test-Path -LiteralPath $sourceSoulDir -PathType Container)) {
        throw "Release asset did not contain .soul/: $asset"
    }

    New-Item -ItemType Directory -Path $targetSoulDir -Force | Out-Null
    Get-ChildItem -LiteralPath $sourceSoulDir -Force | Copy-Item -Destination $targetSoulDir -Recurse -Force
} finally {
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

function Set-JsonProperty {
    param(
        [Parameter(Mandatory = $true)] $Object,
        [Parameter(Mandatory = $true)] [string] $Name,
        [Parameter(Mandatory = $true)] $Value
    )

    $propertyNames = @($Object.PSObject.Properties | ForEach-Object { $_.Name })
    if ($propertyNames -contains $Name) {
        $Object.$Name = $Value
    } else {
        Add-Member -InputObject $Object -MemberType NoteProperty -Name $Name -Value $Value
    }
}

if (-not $SkipOpencodeConfig) {
    $configDir = Split-Path -Parent $ConfigPath
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null

    if ((Test-Path -LiteralPath $ConfigPath -PathType Leaf) -and ((Get-Item -LiteralPath $ConfigPath).Length -gt 0)) {
        try {
            $config = Get-Content -LiteralPath $ConfigPath -Raw | ConvertFrom-Json
        } catch {
            throw "Refusing to edit invalid JSON config at ${ConfigPath}: $($_.Exception.Message)"
        }

        if ($config -isnot [pscustomobject]) {
            throw "Refusing to edit non-object JSON config at $ConfigPath"
        }

        Copy-Item -LiteralPath $ConfigPath -Destination "$ConfigPath.bak.$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())" -Force
    } else {
        $config = [pscustomobject]@{}
    }

    $configPropertyNames = @($config.PSObject.Properties | ForEach-Object { $_.Name })

    if ($configPropertyNames -notcontains '$schema') {
        Set-JsonProperty -Object $config -Name '$schema' -Value 'https://opencode.ai/config.json'
    }

    $configPropertyNames = @($config.PSObject.Properties | ForEach-Object { $_.Name })

    if (($configPropertyNames -notcontains 'mcp') -or ($null -eq $config.mcp)) {
        Set-JsonProperty -Object $config -Name 'mcp' -Value ([pscustomobject]@{})
    }

    if ($config.mcp -isnot [pscustomobject]) {
        throw "Refusing to overwrite non-object 'mcp' value in $ConfigPath"
    }

    $soulExe = Join-Path $targetSoulDir "soul.exe"
    $soulMcp = [pscustomobject]@{
        type = "local"
        command = @($soulExe, "serve", "--root", ".")
        enabled = $true
    }

    Set-JsonProperty -Object $config.mcp -Name 'soul' -Value $soulMcp
    $config | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $ConfigPath -Encoding UTF8
}

"Installed Soul runtime to $targetSoulDir"
if (-not $SkipOpencodeConfig) {
    "Updated opencode MCP config at $ConfigPath"
}
"Restart opencode for the MCP config change to take effect."
