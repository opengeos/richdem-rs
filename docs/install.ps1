# Installs the richdem command-line tool from its GitHub release, on Windows:
#
#     powershell -ExecutionPolicy ByPass -c "irm https://richdem-rs.opengeos.org/install.ps1 | iex"
#
# Settings, from the environment:
#   RICHDEM_VERSION         the release to install (0.1.0 or v0.1.0); the latest by default
#   RICHDEM_INSTALL_DIR     where to put the binary; %USERPROFILE%\.local\bin by default
#   RICHDEM_DOWNLOAD_URL    the directory the archives are downloaded from, for a mirror; the
#                           GitHub release by default
#   RICHDEM_NO_MODIFY_PATH  set to leave the user's PATH as it is
#
# On Linux and macOS, use install.sh.

function Install-Richdem {
    $ErrorActionPreference = "Stop"
    # Invoke-WebRequest's progress bar slows downloads down many times in Windows PowerShell.
    $ProgressPreference = "SilentlyContinue"
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = "opengeos/richdem-rs"
    $arch = $env:PROCESSOR_ARCHITECTURE
    try { $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() } catch {}
    switch ($arch) {
        { $_ -in "X64", "AMD64" } { $target = "x86_64-pc-windows-msvc" }
        "ARM64" { $target = "aarch64-pc-windows-msvc" }
        default { throw "No richdem binary for $arch; install it with pip install richdem-rs or cargo install richdem-cli" }
    }

    $version = $env:RICHDEM_VERSION
    if (-not $version) {
        try {
            $version = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest").tag_name
        } catch {
            throw "Could not find the latest release of $repo; set RICHDEM_VERSION to the version to install"
        }
    }
    $version = "v" + $version.TrimStart("v")
    if ($version -notmatch '^v[0-9][0-9A-Za-z._+-]*$') {
        throw "Invalid version: $version"
    }

    $installDir = $env:RICHDEM_INSTALL_DIR
    if (-not $installDir) { $installDir = Join-Path $env:USERPROFILE ".local\bin" }
    $base = $env:RICHDEM_DOWNLOAD_URL
    if (-not $base) { $base = "https://github.com/$repo/releases/download/$version" }
    $base = $base.TrimEnd("/")
    $name = "richdem-cli-$version-$target"

    $tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "richdem-installer: downloading richdem $version for $target"
        $zip = Join-Path $tmp "$name.zip"
        try {
            Invoke-WebRequest "$base/$name.zip" -OutFile $zip -UseBasicParsing
            Invoke-WebRequest "$base/$name.zip.sha256" -OutFile "$zip.sha256" -UseBasicParsing
        } catch {
            throw "Could not download richdem $version for $target ($($_.Exception.Message)); install it with pip install richdem-rs or cargo install richdem-cli"
        }
        $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split "\s+")[0]
        if ($expected -notmatch '^[0-9a-fA-F]{64}$') {
            throw "Invalid checksum format in $name.zip.sha256"
        }
        if ((Get-FileHash $zip -Algorithm SHA256).Hash -ne $expected.ToUpper()) {
            throw "Checksum mismatch for $name.zip"
        }

        Expand-Archive $zip -DestinationPath $tmp
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
        Copy-Item (Join-Path $tmp "$name\richdem.exe") (Join-Path $installDir "richdem.exe") -Force
        Write-Host "richdem-installer: installed $(Join-Path $installDir 'richdem.exe')"
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    if ($env:RICHDEM_NO_MODIFY_PATH) {
        if (-not (($env:Path -split ";") -contains $installDir)) {
            Write-Host "richdem-installer: $installDir is not on your PATH"
        }
    } else {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if (-not (($userPath -split ";") -contains $installDir)) {
            [Environment]::SetEnvironmentVariable("Path", (@($installDir) + ($userPath -split ";" | Where-Object { $_ })) -join ";", "User")
            Write-Host "richdem-installer: added $installDir to your PATH"
        }
        # This session too (the script runs in the caller's, with irm | iex), which may predate
        # the user PATH having it.
        if (-not (($env:Path -split ";") -contains $installDir)) { $env:Path = "$installDir;$env:Path" }
    }
    & (Join-Path $installDir "richdem.exe") --version
}

Install-Richdem
