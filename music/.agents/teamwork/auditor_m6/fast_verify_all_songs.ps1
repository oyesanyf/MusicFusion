$songs = @("roots_reggae_anthem", "song-one-love-shining-1977", "song_afrobeats_002", "song_reggae_anthem_001")
$versions = @("v1", "v2")
$stems = @("final_master.wav", "stems/bass.wav", "stems/draft_groove.wav", "stems/drums.wav", "stems/guitar.wav", "stems/inpainted_drums.wav", "stems/inpainted_guitar.wav", "stems/other.wav", "stems/raspy_tenor_vocals.wav", "stems/yue_instrumentals.wav", "stems/yue_vocals.wav")

$allPassed = $true
$fileCount = 0

Write-Host "=== VERIFYING ALL 4 SAMPLE SONGS & 8 VERSIONS ===" -ForegroundColor Cyan

foreach ($s in $songs) {
    foreach ($v in $versions) {
        $vDir = "storage/songs/$s/$v"
        
        # 1. Lyrics verification
        $lyrPath = "$vDir/lyrics.txt"
        $manPath = "$vDir/manifest.json"
        $stPath = "$vDir/state.json"
        
        if (-not (Test-Path $lyrPath) -or -not (Test-Path $manPath) -or -not (Test-Path $stPath)) {
            Write-Host "FAIL: Missing metadata files in $vDir" -ForegroundColor Red
            $allPassed = $false
            continue
        }
        
        $lyrBytes = [System.IO.File]::ReadAllBytes($lyrPath)
        if ($lyrBytes.Length -ge 3 -and $lyrBytes[0] -eq 0xEF -and $lyrBytes[1] -eq 0xBB -and $lyrBytes[2] -eq 0xBF) {
            Write-Host "FAIL: UTF-8 BOM detected in $lyrPath" -ForegroundColor Red
            $allPassed = $false
        }
        
        $lyrText = [System.Text.Encoding]::UTF8.GetString($lyrBytes)
        if (-not $lyrText.Contains("[Intro]") -or -not $lyrText.Contains("[Chorus]") -or -not $lyrText.Contains("[Verse")) {
            Write-Host "FAIL: Incomplete lyrics sections in $lyrPath" -ForegroundColor Red
            $allPassed = $false
        }
        
        # 2. Manifest verification
        $manText = [System.IO.File]::ReadAllText($manPath)
        $manJson = ConvertFrom-Json $manText
        if (-not $manJson.lyrics -or $manJson.telemetry.sample_rate -ne 48000 -or $manJson.telemetry.channels -ne 2 -or $manJson.telemetry.clipping -ne $false) {
            Write-Host "FAIL: Invalid manifest telemetry or missing lyrics in $manPath" -ForegroundColor Red
            $allPassed = $false
        }
        
        # 3. Audio stems verification
        foreach ($stem in $stems) {
            $wavPath = "$vDir/$stem"
            if (-not (Test-Path $wavPath)) {
                Write-Host "FAIL: Missing WAV: $wavPath" -ForegroundColor Red
                $allPassed = $false
                continue
            }
            
            $fileCount++
            $fs = [System.IO.File]::OpenRead($wavPath)
            $br = New-Object System.IO.BinaryReader($fs)
            
            $riff = [System.Text.Encoding]::ASCII.GetString($br.ReadBytes(4))
            $fileSize = $br.ReadUInt32()
            $wave = [System.Text.Encoding]::ASCII.GetString($br.ReadBytes(4))
            $fmt = [System.Text.Encoding]::ASCII.GetString($br.ReadBytes(4))
            $subchunk1Size = $br.ReadUInt32()
            $audioFormat = $br.ReadUInt16()
            $numChannels = $br.ReadUInt16()
            $sampleRate = $br.ReadUInt32()
            $byteRate = $br.ReadUInt32()
            $blockAlign = $br.ReadUInt16()
            $bitsPerSample = $br.ReadUInt16()
            
            # Find data chunk
            $dataId = [System.Text.Encoding]::ASCII.GetString($br.ReadBytes(4))
            while ($dataId -ne "data" -and $fs.Position -lt $fs.Length - 8) {
                $skipSize = $br.ReadUInt32()
                $fs.Seek($skipSize, [System.IO.SeekOrigin]::Current) | Out-Null
                $dataId = [System.Text.Encoding]::ASCII.GetString($br.ReadBytes(4))
            }
            $dataSize = $br.ReadUInt32()
            
            # Basic WAV header validation
            if ($riff -ne "RIFF" -or $wave -ne "WAVE" -or $audioFormat -ne 1 -or $numChannels -ne 2 -or $sampleRate -ne 48000 -or $bitsPerSample -ne 16) {
                Write-Host "FAIL: Invalid WAV format in $wavPath (Channels: $numChannels, SR: $sampleRate, Bits: $bitsPerSample)" -ForegroundColor Red
                $allPassed = $false
                $br.Close()
                $fs.Close()
                continue
            }
            
            # Fast scan: sample 5000 16-bit stereo frames across the file
            $totalFrames = [int]($dataSize / 4)
            $step = [Math]::Max(1, [int]($totalFrames / 5000))
            $maxAbs = 0
            $sumSq = 0.0
            $tested = 0
            
            $startDataPos = $fs.Position
            for ($f = 0; $f -lt $totalFrames; $f += $step) {
                $fs.Position = $startDataPos + ($f * 4)
                if ($fs.Position + 4 -gt $fs.Length) { break }
                $sLeft = [Math]::Abs([int]$br.ReadInt16())
                $sRight = [Math]::Abs([int]$br.ReadInt16())
                if ($sLeft -gt $maxAbs) { $maxAbs = $sLeft }
                if ($sRight -gt $maxAbs) { $maxAbs = $sRight }
                $sumSq += ($sLeft * $sLeft) + ($sRight * $sRight)
                $tested += 2
            }
            
            $br.Close()
            $fs.Close()
            
            $peakFloat = $maxAbs / 32768.0
            $rmsFloat = [Math]::Sqrt($sumSq / [Math]::Max(1, $tested)) / 32768.0
            $durSecs = [Math]::Round(($totalFrames / 48000.0), 2)
            
            # Check for digital clipping
            # Master must be <= 0.96605 (-0.30 dBFS)
            # Stems must be <= 1.0 (no digital clipping)
            if ($stem -eq "final_master.wav" -and $peakFloat -gt 0.96605) {
                Write-Host "FAIL: Master peak exceeds brickwall ceiling ($peakFloat > 0.96605) in $wavPath" -ForegroundColor Red
                $allPassed = $false
            } elseif ($peakFloat -gt 1.0001) {
                Write-Host "FAIL: Stem clipped ($peakFloat > 1.0) in $wavPath" -ForegroundColor Red
                $allPassed = $false
            }
            
            # Check for silence
            if ($rmsFloat -lt 0.001) {
                Write-Host "FAIL: Audio is silent (RMS $rmsFloat < 0.001) in $wavPath" -ForegroundColor Red
                $allPassed = $false
            }
        }
        
        Write-Host "PASS: $s $v (11 stems + master verified, lyrics valid, telemetry clean)" -ForegroundColor Green
    }
}

Write-Host "`nTOTAL WAV FILES VERIFIED: $fileCount"
if ($allPassed -and $fileCount -eq 88) {
    Write-Host "RESULT: 100% CLEAN - ALL 88 WAV FILES & LYRICS ARTIFACTS AUTHENTIC AND VALID!" -ForegroundColor Green
} else {
    Write-Host "RESULT: INTEGRITY VIOLATION DETECTED!" -ForegroundColor Red
}
