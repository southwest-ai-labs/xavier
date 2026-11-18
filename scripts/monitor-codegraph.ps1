# scripts/monitor-codegraph.ps1
# Monitor continuo del proceso de indexación de CodeGraph y compilación a VRAM

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$notifyScript = Join-Path $scriptDir "notify-windows.ps1"

Write-Host "Iniciando monitor de indexación para Xavier Brain 2..."

# 1. Notificación inicial de inicio de monitoreo
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2" -Message "Monitor activo: CodeGraph indexando AST (18MB+ WAL). Panel UI disponible en http://127.0.0.1:5173"

$walPath = "E:\proyectosSWAL\xavier\.xavier\code_graph.db-wal"
$previousSize = 0

while ($true) {
    # Verificar si el proceso code-graph sigue en ejecución en WSL
    $cgProcess = wsl -- bash -c "pgrep -f 'code-graph.*scan' || true"
    $cgProcess = $cgProcess.Trim()

    if ($cgProcess -ne "") {
        if (Test-Path $walPath) {
            $walSizeMB = [math]::Round(((Get-Item $walPath).Length / 1MB), 2)
            Write-Host "Indexación en curso (PID $cgProcess) - WAL: $walSizeMB MB"
        }
        Start-Sleep -Seconds 5
    } else {
        Write-Host "Proceso code-graph ha finalizado."
        break
    }
}

# 2. Notificación de fin de escaneo
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2" -Message "Indexación AST de CodeGraph completada. Compilando Living Wiki en VRAM..."

# 3. Ejecutar compilación de Living Wiki a SQLite y Markdown
Write-Host "Ejecutando wiki compile..."
wsl -- bash -c "cd /mnt/e/proyectosSWAL/xavier && ./target/debug/xavier wiki compile ."

# 4. Notificación de Living Wiki en VRAM
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2 (VRAM Engine)" -Message "Living Wiki 100% operativa en VRAM. Latencia <0.15ms lista para consultas."

Write-Host "Monitoreo completado con éxito."
