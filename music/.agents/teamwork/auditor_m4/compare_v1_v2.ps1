$v1 = [System.IO.File]::ReadAllBytes("storage\songs\roots_reggae_anthem\v1\final_master.wav")
$v2 = [System.IO.File]::ReadAllBytes("storage\songs\roots_reggae_anthem\v2\final_master.wav")

$diffCount = 0
for ($i = 44; $i -lt [Math]::Min($v1.Length, $v2.Length); $i++) {
    if ($v1[$i] -ne $v2[$i]) {
        $diffCount++
    }
}

Write-Output "roots_reggae_anthem: Total audio bytes=$($v1.Length - 44), Different bytes=$diffCount"

$s1 = [System.IO.File]::ReadAllBytes("storage\songs\song-one-love-shining-1977\v1\final_master.wav")
$s2 = [System.IO.File]::ReadAllBytes("storage\songs\song-one-love-shining-1977\v2\final_master.wav")

$diffCount2 = 0
for ($i = 44; $i -lt [Math]::Min($s1.Length, $s2.Length); $i++) {
    if ($s1[$i] -ne $s2[$i]) {
        $diffCount2++
    }
}

Write-Output "song-one-love-shining-1977: Total audio bytes=$($s1.Length - 44), Different bytes=$diffCount2"
