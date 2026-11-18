$wscript = New-Object -ComObject WScript.Shell
$shortcuts = @(
    "C:\Users\belal\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Xavier\Xavier.lnk",
    "C:\Users\belal\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Xavier\Xavier TUI.lnk",
    "C:\Users\belal\Desktop\Xavier Brain 2.lnk"
)

foreach ($path in $shortcuts) {
    if (Test-Path $path) {
        $sc = $wscript.CreateShortcut($path)
        Write-Host "=== Shortcut: $path ==="
        Write-Host "Target:   $($sc.TargetPath)"
        Write-Host "Args:     $($sc.Arguments)"
        Write-Host "WorkDir:  $($sc.WorkingDirectory)"
        Write-Host ""
    }
}
