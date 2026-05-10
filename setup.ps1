$TargetDir = $PSScriptRoot
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

if ($UserPath -split ';' -contains $TargetDir) {
    Write-Host "[SixC] Path already exists: $TargetDir" -ForegroundColor Cyan
}
else {
    $NewPath = "$UserPath;$TargetDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Host "[SixC] Added $TargetDir to User Path." -ForegroundColor Green
    Write-Host "[SixC] PLEASE RESTART YOUR TERMINAL to use the 'six' command." -ForegroundColor Yellow
}
