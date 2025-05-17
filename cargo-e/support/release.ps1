# Step 1: Run release-plz to bump, commit, and tag
release-plz update --allow-dirty

# Step 2: Get the 7-character short SHA of HEAD
$sha = git rev-parse --short=7 HEAD

# Step 3: Extract version from Cargo.toml
$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' |
            ForEach-Object { $_.Matches[0].Groups[1].Value })

# Step 4: Read CHANGELOG.md lines
$lines = Get-Content -Path CHANGELOG.md

# Step 5: Find the first version header (skipping [Unreleased])
$startIndex = $null
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $startIndex = $i
        break
    }
}

if ($startIndex -eq $null) {
    Write-Error "Could not find a version header in CHANGELOG.md"
    exit 1
}

# Step 6: Find the end of the current section (next version header or end of file)
$endIndex = $lines.Count
for ($i = $startIndex + 1; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^## \[\d+\.\d+\.\d+\]') {
        $endIndex = $i
        break
    }
}

# Step 7: Extract changelog body
$changelogBodyLines = $lines[$startIndex..($endIndex - 1)]
$changelogBody = $changelogBodyLines -join "`n"

# Step 8: Extract date from version header
if ($lines[$startIndex] -match '- (\d{4})-(\d{2})-(\d{2})') {
    $year = $matches[1].Substring(2)
    $month = $matches[2]
    $day = $matches[3]
    $date = "$year/$month/$day"
} else {
    $date = Get-Date -Format 'yy/MM/dd'
}

# Step 9: Build LAST_RELEASE content
$content = "$date|$sha|$version`n$changelogBody"

# Step 10: Write to LAST_RELEASE with UTF8 encoding, no trailing newline
Set-Content -Path LAST_RELEASE -Value $content -Encoding UTF8

# Step 11: Amend the last commit to include LAST_RELEASE
git add LAST_RELEASE
git commit --amend --no-edit
