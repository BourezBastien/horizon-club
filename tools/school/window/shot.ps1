# Save a window's picture to a PNG file, even when another window covers it.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File shot.ps1 -Process HorizonClubLauncher -Out window.png [-Wait 30] [-Id 1234]
#
# -Process  the name of the process whose main window is captured
# -Out      the PNG file to write (its folder has to exist)
# -Wait     seconds to wait for the window, at most (default 20)
# -Id       the process itself, when several of one name run (a test's beside the player's own)
#
# The window is captured through PrintWindow with PW_RENDERFULLCONTENT, so what is on the rest
# of the screen stays private, and the parts of a window that hang off the screen are in the
# picture too. What the window does not show (below a scroll) is not: post.ps1 scrolls it, or
# shift.ps1 makes the window taller. Measure the points to click on this picture, from its
# top-left corner.
[CmdletBinding()]
param(
    [string]$Process = 'HorizonClubLauncher',
    [Parameter(Mandatory = $true)][string]$Out,
    [int]$Wait = 20,
    [int]$Id = 0
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Shot {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr device, uint flags);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
}
'@
[void][Shot]::SetProcessDPIAware()

$deadline = (Get-Date).AddSeconds($Wait)
$handle = [IntPtr]::Zero
while ($handle -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline) {
    if ($Id -gt 0) {
        $found = Get-Process -Id $Id -ErrorAction SilentlyContinue
        # The process is there: its window is waited for, not the process.
        if ($found -and $found.MainWindowHandle -eq 0) { $found = $null; Start-Sleep -Milliseconds 400; continue }
    } else {
        $found = Get-Process -Name $Process -ErrorAction SilentlyContinue |
            Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    }
    if ($found) { $handle = $found.MainWindowHandle }
    else { Start-Sleep -Milliseconds 400 }
}
if ($handle -eq [IntPtr]::Zero) { throw "No window of $Process in $Wait seconds." }

# A window that starts minimised answers -32000,-32000 for its rectangle: restore it first.
[void][Shot]::ShowWindow($handle, 9)
Start-Sleep -Milliseconds 800
$rect = New-Object Shot+RECT
if (-not [Shot]::GetWindowRect($handle, [ref]$rect)) { throw 'GetWindowRect failed.' }

$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$device = $graphics.GetHdc()
$printed = [Shot]::PrintWindow($handle, $device, 2)
$graphics.ReleaseHdc($device)
$graphics.Dispose()
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bitmap.Dispose()
Write-Output "Window: $width x $height at $($rect.Left),$($rect.Top); captured: $printed; saved: $Out"
