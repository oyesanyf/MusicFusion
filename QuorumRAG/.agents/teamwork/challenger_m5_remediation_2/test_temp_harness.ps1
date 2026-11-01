$verifyHarnessDir = Join-Path $env:TEMP "quorumrag_test_harness"
if (Test-Path $verifyHarnessDir) { Remove-Item -Path $verifyHarnessDir -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $verifyHarnessDir "src") -Force | Out-Null
$harnessCargo = @"
[package]
name = "verify-codegen"
version = "0.1.0"
edition = "2021"

[dependencies]
quorum-coordinator = { path = "d:/harfile/QuorumRAG/crates/quorum-coordinator" }
"@
Set-Content -Path (Join-Path $verifyHarnessDir "Cargo.toml") -Value $harnessCargo
$harnessMain = @"
pub struct RrfCombiner;
impl RrfCombiner {
    pub fn compute_rrf_score(ranks: &[usize], k: f32) -> f32 {
        ranks.iter().map(|&r| 1.0 / (k + r as f32)).sum()
    }
}
fn main() {}
"@
Set-Content -Path (Join-Path $verifyHarnessDir "src\main.rs") -Value $harnessMain
$out = cargo check --manifest-path (Join-Path $verifyHarnessDir "Cargo.toml") 2>&1
Write-Host "ExitCode: $LASTEXITCODE"
Write-Host "Output: $out"
Remove-Item -Path $verifyHarnessDir -Recurse -Force -ErrorAction SilentlyContinue
