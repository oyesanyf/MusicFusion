$paths = @(
    "storage\songs\roots_reggae_anthem\v1\final_master.wav",
    "storage\songs\roots_reggae_anthem\v2\final_master.wav",
    "storage\songs\song-one-love-shining-1977\v1\final_master.wav",
    "storage\songs\song-one-love-shining-1977\v2\final_master.wav"
)

foreach ($p in $paths) {
    if (Test-Path $p) {
        $fileStream = [System.IO.File]::OpenRead($p)
        $header = New-Object byte[] 44
        $read = $fileStream.Read($header, 0, 44)
        $riff = [System.Text.Encoding]::ASCII.GetString($header[0..3])
        $wave = [System.Text.Encoding]::ASCII.GetString($header[8..11])
        $channels = [System.BitConverter]::ToInt16($header, 22)
        $sampleRate = [System.BitConverter]::ToInt32($header, 24)
        $bitsPerSample = [System.BitConverter]::ToInt16($header, 34)
        $fileLen = $fileStream.Length

        # Sample some data bytes to confirm non-zero audio content
        $sampleBuffer = New-Object byte[] 1024
        $fileStream.Seek(1000, [System.IO.SeekOrigin]::Begin) | Out-Null
        $samplesRead = $fileStream.Read($sampleBuffer, 0, 1024)
        $nonZeroCount = 0
        foreach ($b in $sampleBuffer) {
            if ($b -ne 0) { $nonZeroCount++ }
        }
        $fileStream.Close()

        Write-Output "$p : Length=$fileLen, RIFF=$riff, WAVE=$wave, Channels=$channels, SampleRate=$sampleRate, Bits=$bitsPerSample, NonZero1024=$nonZeroCount"
    } else {
        Write-Output "$p : NOT FOUND"
    }
}
