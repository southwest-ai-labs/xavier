# scripts/sync-knowledge.ps1
# Sincronización rápida de la Base de Conocimiento de Xavier (Cloud Capsule o Git Delta)

param(
    [switch]$FromCloud,
    [switch]$FromGit
)

$ErrorActionPreference = "Stop"
$xavierDir = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "..\.xavier"

Write-Host "=== Sincronización de Base de Conocimiento Xavier Brain 2 ===" -ForegroundColor Cyan

if ($FromCloud) {
    Write-Host "Descargando snapshot pre-compilado de CI (GitHub Actions / Cloud)..." -ForegroundColor Yellow
    # Descarga el archivo comprimido generado por el workflow de GitHub Actions
    $repo = "iberi22/xavier"
    $artifactUrl = "https://github.com/$repo/releases/download/latest/xavier-brain2-knowledge.tar.gz"
    
    $dest = Join-Path $env:TEMP "xavier-brain2-knowledge.tar.gz"
    try {
        Invoke-WebRequest -Uri $artifactUrl -OutFile $dest
        tar -xzf $dest -C (Split-Path -Parent $xavierDir)
        Write-Host "[OK] Base de conocimiento restaurada desde el CI con éxito." -ForegroundColor Green
    } catch {
        Write-Warning "No se pudo descargar el release asset. Asegúrate de que el workflow de CI haya publicado el snapshot."
    }
} elseif ($FromGit) {
    Write-Host "Ejecutando sincronización incremental basada en Git Delta..." -ForegroundColor Yellow
    # Solo procesa los archivos cambiados en el working tree o en el último commit
    wsl -- bash -c "cd /mnt/e/proyectosSWAL/xavier && ./target/debug/xavier code sync --git"
    Write-Host "[OK] Sincronización incremental completada en milisegundos." -ForegroundColor Green
} else {
    Write-Host "Uso:"
    Write-Host "  .\scripts\sync-knowledge.ps1 -FromCloud  # Descarga base de datos pre-indexada de CI"
    Write-Host "  .\scripts\sync-knowledge.ps1 -FromGit    # Sincroniza micro-cambios locales con Git diff"
}
