# scripts/window-manager.ps1
# Herramienta de gestión de ventanas de Windows 11 para Computer Use

param(
    [ValidateSet("List", "Focus", "Minimize", "Maximize")]
    [string]$Action = "List",
    [string]$Title = ""
)

Add-Type @"
    using System;
    using System.Runtime.InteropServices;
    using System.Text;

    public class Win32Window {
        [DllImport("user32.dll")]
        public static extern bool SetForegroundWindow(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

        [DllImport("user32.dll")]
        public static extern bool IsWindowVisible(IntPtr hWnd);

        [DllImport("user32.dll")]
        public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);

        public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

        [DllImport("user32.dll")]
        public static extern bool EnumWindows(EnumWindowsProc enumProc, IntPtr lParam);
    }
"@

switch ($Action) {
    "List" {
        Write-Host "=== Ventanas activas visibles ===" -ForegroundColor Cyan
        Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -ne "" } | 
            Select-Object Id, ProcessName, MainWindowTitle | Format-Table -AutoSize
    }
    "Focus" {
        if ($Title -eq "") {
            Write-Error "Debes especificar -Title para la acción Focus."
            exit 1
        }
        $proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$Title*" -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
        if ($proc) {
            [Win32Window]::ShowWindow($proc.MainWindowHandle, 9) | Out-Null # SW_RESTORE
            [Win32Window]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
            Write-Host "[OK] Ventana '$($proc.MainWindowTitle)' enfocada en primer plano." -ForegroundColor Green
        } else {
            Write-Warning "No se encontró ninguna ventana con título que coincida con '$Title'."
        }
    }
}
