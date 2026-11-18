# scripts/launch-desktop-app.ps1
# Crea el acceso directo de escritorio y abre la ventana nativa de Xavier Brain 2

$desktop = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::Desktop)
$shortcutPath = Join-Path $desktop "Xavier Brain 2.lnk"

$iconPath = "C:\Users\belal\AppData\Local\Programs\Xavier\SouthWest AI Labs\Xavier\panel-ui\favicon.ico"
$token = "8ae8b432a2f42cffcdf26838d9646ab429ca6582f593af66bd42e61dab6991f7"
$appUrl = "http://127.0.0.1:5173/#/session?token=$token"

# Buscar el ejecutable de Edge o Brave
$browserPath = ""
if (Test-Path "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe") {
    $browserPath = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
} elseif (Test-Path "C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe") {
    $browserPath = "C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"
}

if ($browserPath -ne "") {
    $wscript = New-Object -ComObject WScript.Shell
    $shortcut = $wscript.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $browserPath
    $shortcut.Arguments = "--new-window --app=`"$appUrl`""
    if (Test-Path $iconPath) {
        $shortcut.IconLocation = "$iconPath, 0"
    }
    $shortcut.Description = "Xavier Brain 2 - Cognitive Memory Engine"
    $shortcut.Save()
    Write-Host "[OK] Acceso directo creado en: $shortcutPath" -ForegroundColor Green

    # Lanzar la ventana en primer plano
    Write-Host "Lanzando ventana de aplicación dedicada..." -ForegroundColor Cyan
    Start-Process $browserPath -ArgumentList "--new-window", "--app=`"$appUrl`""
    Write-Host "[OK] Ventana lanzada exitosamente." -ForegroundColor Green
} else {
    Write-Warning "No se encontró msedge.exe ni brave.exe"
}
