# scripts/setup-windows-build.ps1
# Script de preparación del entorno nativo de compilación en Windows 11

$ErrorActionPreference = "Continue"

Write-Host "=== Diagnóstico y Configuración de Entorno Nativo Windows 11 ===" -ForegroundColor Cyan

# 1. PATH Setup
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $cargoBin) {
    Write-Host "[OK] Directorio de Cargo detectado: $cargoBin" -ForegroundColor Green
    $currentPath = [System.Environment]::GetEnvironmentVariable("Path", [System.EnvironmentVariableTarget]::User)
    if ($currentPath -notlike "*$cargoBin*") {
        [System.Environment]::SetEnvironmentVariable("Path", ($currentPath.TrimEnd(";") + ";$cargoBin"), [System.EnvironmentVariableTarget]::User)
        Write-Host "[OK] Cargo bin agregado permanentemente al PATH del usuario." -ForegroundColor Green
    } else {
        Write-Host "[INFO] Cargo bin ya se encuentra en el PATH del usuario." -ForegroundColor Gray
    }
    $env:Path = "$cargoBin;$env:Path"
} else {
    Write-Warning "No se encontró $cargoBin"
}

# 2. Versiones de Rust
Write-Host "`n--- Herramientas de Rust en Windows ---" -ForegroundColor Cyan
& "$cargoBin\rustc.exe" --version
& "$cargoBin\cargo.exe" --version
& "$cargoBin\rustup.exe" show

# 3. Diagnóstico de Tauri CLI
Write-Host "`n--- Diagnóstico de Tauri CLI en panel-ui ---" -ForegroundColor Cyan
Set-Location "E:\proyectosSWAL\xavier\panel-ui"
pnpm tauri info
