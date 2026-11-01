$songs = @("roots_reggae_anthem", "song-one-love-shining-1977", "song_afrobeats_002", "song_reggae_anthem_001")
$versions = @("v1", "v2")
$stems = @("final_master.wav", "stems/bass.wav", "stems/draft_groove.wav", "stems/drums.wav", "stems/guitar.wav", "stems/inpainted_drums.wav", "stems/inpainted_guitar.wav", "stems/other.wav", "stems/raspy_tenor_vocals.wav", "stems/yue_instrumentals.wav", "stems/yue_vocals.wav")

$results = @()
$lyricsChecks = @()

foreach ($s in $songs) {
    foreach ($v in $versions) {
        $vDir = "storage/songs/$s/$v"
        
        # Check lyrics
        $lyrPath = "$vDir/lyrics.txt"
        $manPath = "$vDir/manifest.json"
        $stPath = "$vDir/state.json"
        
        $hasLyr = Test-Path $lyrPath
        $hasMan = Test-Path $manPath
        $hasSt = Test-Path $stPath
        
        $lyrContent = if ($hasLyr) { [System.IO.File]::ReadAllText($lyrPath) } else { "" }
        $manContent = if ($hasMan) { [System.IO.File]::ReadAllText($manPath) } else { "" }
        $stContent = if ($hasSt) { [System.IO.File]::ReadAllText($stPath) } else { "" }
        
        $hasIntro = $lyrContent.Contains("[Intro]")
        $hasChorus = $lyrContent.Contains("[Chorus]")
        $hasVerse = $lyrContent.Contains("[Verse")
        
        # Check UTF-8 BOM
        $lyrBytes = if ($hasLyr) { [System.IO.File]::ReadAllBytes($lyrPath) } else { @() }
        $hasBom = ($lyrBytes.Length -ge 3 -and $lyrBytes[0] -eq 0xEF -and $lyrBytes[1] -eq 0xBB -and $lyrBytes[2] -eq 0xBF)
        
        $lyricsChecks += [PSCustomObject]@{
            Song = $s
            Version = $v
            LyricsFile = $hasLyr
            ManifestFile = $hasMan
            StateFile = $hasSt
            HasIntro = $hasIntro
            HasChorus = $hasChorus
            HasVerse = $hasVerse
            HasBOM = $hasBom
            LyricsLen = $lyrContent.Length
        }
        
        # Check WAV files
        foreach ($stem in $stems) {
            $wavPath = "$vDir/$stem"
            if (-not (Test-Path $wavPath)) {
                $results += [PSCustomObject]@{
                    Song = $s
                    Version = $v
                    Stem = $stem
                    Exists = $false
                    ValidHeader = $false
                    SampleRate = 0
                    Channels = 0
                    Bits = 0
                    Duration = 0
                    Peak = 0
                    RMS = 0
                    Clipping = $true
                }
                continue
            }
            
            $bytes = [System.IO.File]::ReadAllBytes($wavPath)
            if ($bytes.Length -lt 44) {
                $results += [PSCustomObject]@{
                    Song = $s
                    Version = $v
                    Stem = $stem
                    Exists = $true
                    ValidHeader = $false
                    SampleRate = 0
                    Channels = 0
                    Bits = 0
                    Duration = 0
                    Peak = 0
                    RMS = 0
                    Clipping = $true
                }
                continue
            }
            
            $riff = [System.Text.Encoding]::ASCII.GetString($bytes, 0, 4)
            $wave = [System.Text.Encoding]::ASCII.GetString($bytes, 8, 4)
            $channels = [BitConverter]::ToUInt16($bytes, 22)
            $sr = [BitConverter]::ToUInt32($bytes, 24)
            $bits = [BitConverter]::ToUInt16($bytes, 34)
            
            # Find data chunk
            $dataOffset = 36
            while ($dataOffset + 8 -le $bytes.Length) {
                $chunkId = [System.Text.Encoding]::ASCII.GetString($bytes, $dataOffset, 4)
                if ($chunkId -eq "data") { break }
                $chunkSize = [BitConverter]::ToUInt32($bytes, $dataOffset + 4)
                $dataOffset += 8 + $chunkSize
            }
            
            $dataSize = [BitConverter]::ToUInt32($bytes, $dataOffset + 4)
            $dataStart = $dataOffset + 8
            $numSamples = [int]($dataSize / 2) # 16-bit
            
            # Sample peak and RMS over up to 100,000 samples
            $step = [Math]::Max(1, [int]($numSamples / 100000))
            $maxVal = 0.0
            $sumSq = 0.0
            $sampleCount = 0
            
            for ($i = 0; $i -lt $numSamples; $i += $step) {
                $idx = $dataStart + ($i * 2)
                if ($idx + 1 -ge $bytes.Length) { break }
                $val = [BitConverter]::ToInt16($bytes, $idx) / 32768.0
                $absVal = [Math]::Abs($val)
                if ($absVal -gt $maxVal) { $maxVal = $absVal }
                $sumSq += $val * $val
                $sampleCount++
            }
            
            $rms = [Math]::Sqrt($sumSq / [Math]::Max(1, $sampleCount))
            $dur = ($numSamples / ($channels * $sr))
            $isClipped = if ($stem -eq "final_master.wav") { $maxVal -gt 0.96605 } else { $maxVal -gt 1.0001 }
            
            $results += [PSCustomObject]@{
                Song = $s
                Version = $v
                Stem = $stem
                Exists = $true
                ValidHeader = ($riff -eq "RIFF" -and $wave -eq "WAVE")
                SampleRate = $sr
                Channels = $channels
                Bits = $bits
                Duration = [Math]::Round($dur, 2)
                Peak = [Math]::Round($maxVal, 4)
                RMS = [Math]::Round($rms, 4)
                Clipping = $isClipped
            }
        }
    }
}

Write-Host "=== LYRICS & METADATA SUMMARY ==="
$lyricsChecks | Format-Table -AutoSize

Write-Host "=== AUDIO FILES SUMMARY ==="
Write-Host "Total WAV files analyzed: $($results.Count)"
$failedWavs = $results | Where-Object { -not $_.Exists -or -not $_.ValidHeader -or $_.SampleRate -ne 48000 -or $_.Channels -ne 2 -or $_.Bits -ne 16 -or $_.Clipping -or $_.RMS -lt 0.001 }
if ($failedWavs.Count -gt 0) {
    Write-Host "FAILURES DETECTED:"
    $failedWavs | Format-Table -AutoSize
} else {
    Write-Host "ALL 88 WAV FILES PASSED INTEGRITY VERIFICATION!"
    Write-Host "`n=== FINAL MASTER SOUND CHECK (All 8 versions) ==="
    $results | Where-Object { $_.Stem -eq "final_master.wav" } | Format-Table -AutoSize
}
