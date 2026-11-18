# scripts/take-screenshot.ps1
# Captura de pantalla nativa de Windows mediante System.Drawing

param(
    [string]$OutputPath = "C:\Users\belal\.gemini\antigravity\brain\70b40135-4b0b-40ae-a388-de843e7b2d27\current_screen.png",
    [string]$WindowTitle = ""
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

if ($WindowTitle -ne "") {
    # Capturar ventana específica si se pasa WindowTitle
    Add-Type @"
        using System;
        using System.Runtime.InteropServices;
        public class Win32 {
            [DllImport("user32.dll")]
            public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
            [DllImport("user32.dll")]
            public static extern bool SetForegroundWindow(IntPtr hWnd);
            [StructLayout(LayoutKind.Sequential)]
            public struct RECT {
                public int Left;
                public int Top;
                public int Right;
                public int Bottom;
            }
        }
"@
    $proc = Get-Process | Where-Object { $_.MainWindowTitle -like "*$WindowTitle*" -and $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if ($proc) {
        $handle = $proc.MainWindowHandle
        [Win32]::SetForegroundWindow($handle) | Out-Null
        Start-Sleep -Milliseconds 250
        $rect = New-Object Win32+RECT
        [Win32]::GetWindowRect($handle, [ref]$rect) | Out-Null
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top

        if ($width -gt 0 -and $height -gt 0) {
            $bmp = New-Object System.Drawing.Bitmap($width, $height)
            $gfx = [System.Drawing.Graphics]::FromImage($bmp)
            $gfx.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size($width, $height)))
            $bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
            $gfx.Dispose()
            $bmp.Dispose()
            Write-Host "Ventana '$($proc.MainWindowTitle)' capturada en: $OutputPath"
            exit 0
        }
    }
}

# Captura de pantalla completa primaria
$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
$bmp.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()

Write-Host "Captura completa guardada en: $OutputPath"
