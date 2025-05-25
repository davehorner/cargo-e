# Step 0: Run release-plz
Write-Host "Running: release-plz update --allow-dirty"
& release-plz update --allow-dirty
if ($LASTEXITCODE -ne 0) {
    Write-Error "release-plz failed"
    exit 1
}

# Step 1: Update README.md with version from Cargo.toml

# Read version from Cargo.toml
$cargoToml = Get-Content "Cargo.toml"
$versionLine = $cargoToml | Where-Object { $_ -match '^\s*version\s*=' }
if (-not $versionLine -or $versionLine -notmatch '=\s*"(.*?)"') {
    Write-Error "Could not find or parse version in Cargo.toml"
    exit 1
}
$version = $matches[1]
Write-Host "Found version: $version"

# Read README.md
$readmePath = "README.md"
$readmeContent = Get-Content $readmePath -Raw

# Compare with ../README.md
$parentPath = "..\README.md"
if (-not (Test-Path $parentPath)) {
    Write-Error "Parent README not found at $parentPath"
    exit 1
}
$parentContent = Get-Content $parentPath -Raw
if ($readmeContent -ne $parentContent) {
    Write-Error "Parent README.md content differs from local README.md. Aborting update."
    exit 1
}

# Replace version in README content
#$newContent = [regex]::Replace($readmeContent, '>(\d+\.\d+\.\d+)<', ">$version<")

# Write updated content to README.md and ../README.md
#[System.IO.File]::WriteAllText($readmePath, $newContent, [System.Text.UTF8Encoding]::new($true))
#[System.IO.File]::WriteAllText($parentPath, $newContent, [System.Text.UTF8Encoding]::new($true))
#Write-Host "Updated README.md and ../README.md with version $version"

# Step 2: Extract git short SHA
$sha = git rev-parse --short=7 HEAD
Write-Host "Found SHA: $sha"

# Step 3: Read and parse CHANGELOG.md
$lines = Get-Content -Path CHANGELOG.md

# Find first version section
$startIndex = $null
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $startIndex = $i
        break
    }
}
if ($startIndex -eq $null) {
    Write-Error "Could not find version section in CHANGELOG.md"
    exit 1
}

# Find end of the section
$endIndex = $lines.Count
for ($i = $startIndex + 1; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $endIndex = $i
        break
    }
}
$changelogBodyLines = $lines[$startIndex..($endIndex - 1)]
$changelogBody = $changelogBodyLines -join "`n"

# Extract date
if ($lines[$startIndex] -match '- (\d{4})-(\d{2})-(\d{2})') {
    $year = $matches[1].Substring(2)
    $month = $matches[2]
    $day = $matches[3]
    $date = "$year/$month/$day"
} else {
    $date = Get-Date -Format 'yy/MM/dd'
}
Write-Host "Using release date: $date"

# Step 4: Write LAST_RELEASE
$lastReleaseContent = "$date|$sha|$version`n$changelogBody"
[System.IO.File]::WriteAllText("LAST_RELEASE", $lastReleaseContent, [System.Text.UTF8Encoding]::new($true))

Write-Host "Wrote LAST_RELEASE"
e_update_readme -p
# Step 5: Amend commit
git add ..\Cargo.lock
git add .\CHANGELOG.md
git add .\Cargo.toml
git add .\README.md
git add ..\README.md
git add LAST_RELEASE
git commit --amend --no-edit
Write-Host "Amended last commit to include LAST_RELEASE"
