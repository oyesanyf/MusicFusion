# Delegate to music\scripts\solo_pr_ship.ps1 or run directly
param(
    [string]$Title = "test: add challenger empirical suites for milestone 8",
    [string]$Body = "",
    [switch]$SkipTest = $false
)
& "$PSScriptRoot\..\music\scripts\solo_pr_ship.ps1" -Title $Title -Body $Body -SkipTest:$SkipTest
