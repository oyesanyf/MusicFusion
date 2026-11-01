@echo off
title MusicFusion Local WAV & Studio Player
set "WAV_PATH=%~1"
if "%WAV_PATH%"=="" set "WAV_PATH=%~dp0storage\songs\roots_reggae_anthem\v1\final_master.wav"
echo ============================================================================
echo   MusicFusion AI Studio - Direct Hardware WAV Player
echo   Playing: %WAV_PATH%
echo ============================================================================
powershell -NoProfile -ExecutionPolicy Bypass -Command "$p = '%WAV_PATH%'; if (Test-Path $p) { Write-Host 'Playing 48 kHz 16-bit PCM Stereo WAV on Default Windows Audio Output...' -ForegroundColor Green; $sp = New-Object System.Media.SoundPlayer $p; $sp.PlaySync() } else { Write-Host 'File not found:' $p -ForegroundColor Red }"
pause
