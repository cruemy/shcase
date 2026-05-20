Add-Type -AssemblyName System.IO.Compression.FileSystem

$zip = [System.IO.Compression.ZipFile]::OpenRead('ejemplo.pptx')

Write-Host "=== Slides ==="
$zip.Entries | Where-Object { $_.FullName -match 'slides/slide\d+\.xml$' -and $_.FullName -notmatch 'Layout|Master|rels' } | Sort-Object FullName | ForEach-Object { Write-Host "  $($_.FullName)" }

Write-Host ""
Write-Host "=== presentation.xml.rels (slide entries) ==="
$e = $zip.GetEntry('ppt/_rels/presentation.xml.rels')
$r = New-Object System.IO.StreamReader($e.Open())
$c = $r.ReadToEnd()
$r.Close()
$c -split ">" | Select-String "slide" | ForEach-Object { Write-Host "  $_>" }

Write-Host ""
Write-Host "=== presentation.xml sldIdLst ==="
$e = $zip.GetEntry('ppt/presentation.xml')
$r = New-Object System.IO.StreamReader($e.Open())
$c = $r.ReadToEnd()
$r.Close()
if ($c -match '(?s)<p:sldIdLst>.*?</p:sldIdLst>') { Write-Host "  $($matches[0])" } else { Write-Host "  (not found)" }

Write-Host ""
Write-Host "=== Content_Types slide entries ==="
$e = $zip.GetEntry('[Content_Types].xml')
$r = New-Object System.IO.StreamReader($e.Open())
$c = $r.ReadToEnd()
$r.Close()
$c -split ">" | Select-String 'slides/slide|notesSlide' | ForEach-Object { Write-Host "  $_>" }

Write-Host ""
Write-Host "=== Notes slides ==="
$zip.Entries | Where-Object { $_.FullName -match 'notesSlide' } | Sort-Object FullName | ForEach-Object { Write-Host "  $($_.FullName) ($($_.Length) bytes)" }

Write-Host ""
Write-Host "=== slide1 content ==="
$e = $zip.GetEntry('ppt/slides/slide1.xml')
$r = New-Object System.IO.StreamReader($e.Open())
$c = $r.ReadToEnd()
$r.Close()
$c -split '<a:t[^>]*>' | Select-Object -Skip 1 | ForEach-Object {
    $text = $_ -replace '</a:t>.*', ''
    if ($text.Trim()) { Write-Host "  a:t = $text" }
}

Write-Host ""
Write-Host "=== slide3 content ==="
$e = $zip.GetEntry('ppt/slides/slide3.xml')
if ($e) {
    $r = New-Object System.IO.StreamReader($e.Open())
    $c = $r.ReadToEnd()
    $r.Close()
    $c -split '<a:t[^>]*>' | Select-Object -Skip 1 | ForEach-Object {
        $text = $_ -replace '</a:t>.*', ''
        if ($text.Trim()) { Write-Host "  a:t = $text" }
    }
}

$zip.Dispose()
