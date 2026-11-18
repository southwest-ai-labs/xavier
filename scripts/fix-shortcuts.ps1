# scripts/fix-shortcuts.ps1
# Actualiza los accesos directos de Xavier en Menú Inicio y Escritorio
# para que abran la Ventana de UI nativa en modo Standalone (sin terminal)

$wscript = New-Object -ComObject WScript.Shell

$edgePath = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
$token = "8ae8b432a2f42cffcdf26838d9646ab429ca6582f593af66bd42e61dab6991f7"
$appUrl = "http://127.0.0.1:5173/#/session?token=$token"
$iconPath = "C:\Users\belal\AppData\Local\Programs\Xavier\SouthWest AI Labs\Xavier\panel-ui\favicon.ico"
if (-not (Test-Path $iconPath)) {
    $iconPath = "E:\proyectosSWAL\xavier\panel-ui\public\favicon.ico"
}

$shortcutsToUpdate = @(
    "C:\Users\belal\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Xavier\Xavier.lnk",
    "C:\Users\belal\Desktop\Xavier Brain 2.lnk",
    "C:\Users\belal\Desktop\Xavier.lnk"
)

foreach ($scPath in $shortcutsToUpdate) {
    $parent = Split-Path -Parent $scPath
    if (-not (Test-Path $parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $sc = $wscript.CreateShortcut($scPath)
    $sc.TargetPath = $edgePath
    $sc.Arguments = "--new-window --app=`"$appUrl`""
    if (Test-Path $iconPath) {
        $sc.IconLocation = "$iconPath, 0"
    }
    $sc.Description = "Xavier Brain 2 — Living Knowledge Engine & Cognitive Core"
    $sc.WorkingDirectory = "E:\proyectosSWAL\xavier"
    $sc.Save()
    Write-Host "[OK] Acceso directo actualizado: $scPath" -ForegroundColor Green
}

# Crear también un lanzador VBS silencioso en el directorio de instalación
$installDir = "C:\Users\belal\AppData\Local\Programs\Xavier\SouthWest AI Labs\Xavier"
if (Test-Path $installDir) {
    $vbsContent = @"
Set WshShell = CreateObject("WScript.Shell")
WshShell.Run """$edgePath"" --new-window --app=""$appUrl""", 1, False
"@
    Set-Content -Path (Join-Path $installDir "Xavier-UI.vbs") -Value $vbsContent
    Write-Host "[OK] Creado lanzador silencioso Xavier-UI.vbs en $installDir" -ForegroundColor Green
}
