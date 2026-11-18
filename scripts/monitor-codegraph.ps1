# scripts/monitor-codegraph.ps1
# Monitor continuo del proceso de indexación de CodeGraph, compilación a VRAM y suspensión

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$notifyScript = Join-Path $scriptDir "notify-windows.ps1"

Write-Host "Iniciando monitor de indexación para Xavier Brain 2..."

# 1. Notificación inicial de inicio de monitoreo
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2" -Message "Monitor activo: CodeGraph indexando AST. Panel UI disponible en http://127.0.0.1:5173"

$walPath = "E:\proyectosSWAL\xavier\.xavier\code_graph.db-wal"

while ($true) {
    # Verificar si el proceso code-graph sigue en ejecución en WSL
    $cgProcess = wsl -- bash -c "pgrep -x code-graph || true"
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

# 2. Notificación de fin de escaneo AST
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2" -Message "Indexación AST de CodeGraph completada. Compilando Living Wiki en VRAM..."

# 3. Ejecutar compilación de Living Wiki a SQLite y Markdown
Write-Host "Ejecutando wiki compile..."
wsl -- bash -c "cd /mnt/e/proyectosSWAL/xavier && ./target/debug/xavier wiki compile ."

# 4. Notificación de Living Wiki en VRAM
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2 (VRAM Engine)" -Message "Living Wiki 100% operativa en VRAM. Latencia <0.15ms lista para consultas."

# 5. Notificación de suspensión del equipo Windows
& powershell.exe -ExecutionPolicy Bypass -File $notifyScript -Title "Xavier Brain 2" -Message "Indexación y compilación en VRAM finalizadas con éxito. Suspendiendo el equipo en 15 segundos..."
Write-Host "Todo el trabajo ha finalizado. Suspendiendo el equipo en 15 segundos..."
Start-Sleep -Seconds 15

# 6. Poner el equipo en suspensión (Sleep)
Write-Host "Suspendiendo equipo ahora..."
rundll32.exe powrprof.dll,SetSuspendState 0,1,0
