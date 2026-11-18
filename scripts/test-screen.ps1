Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.CopyFromScreen(0, 0, 0, 0, (New-Object System.Drawing.Size($bounds.Width, $bounds.Height)))
$outPath = "C:\Users\belal\.gemini\antigravity\brain\70b40135-4b0b-40ae-a388-de843e7b2d27\test_screen.png"
$bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
$gfx.Dispose()
$bmp.Dispose()
Write-Host "Success: $(Test-Path $outPath)"
Write-Host "Size: $((Get-Item $outPath).Length) bytes"
