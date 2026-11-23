# Move a window on the screen, or make it another size, to look at a long page in one picture.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File shift.ps1 -Process HorizonClubLauncher -Up 700 -Width 1196 -Height 1500 [-Id 1234]
#
# -Process  the name of the process whose main window is moved
# -Id       the process itself, when several of one name run (a test's beside the player's own)
# -Up       pixels to slide it up (default 420): a negative number slides it down
# -Width, -Height   a new size (0, the default: unchanged). A window taller than the screen is
#           cut to the largest size Windows allows (about the screen's height); shot.ps1 still
#           pictures the part that hangs off the screen.
#
# The window is not raised and keeps its size unless asked. It ends shifted: close it, or slide
# it back with a negative -Up, afterwards.
[CmdletBinding()]
param(
    [string]$Process = 'HorizonClubLauncher',
    [int]$Up = 420,
    [int]$Width = 0,
    [int]$Height = 0,
    [int]$Id = 0
)
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Shift {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr w, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr w, IntPtr after, int x, int y, int width, int height, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[void][Shift]::SetProcessDPIAware()
$found = if ($Id -gt 0) {
    Get-Process -Id $Id -ErrorAction SilentlyContinue
} else {
    Get-Process -Name $Process -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
}
if (-not $found -or $found.MainWindowHandle -eq 0) { throw "No window of $Process." }
$handle = $found.MainWindowHandle
$rect = New-Object Shift+RECT
if (-not [Shift]::GetWindowRect($handle, [ref]$rect)) { throw 'GetWindowRect failed.' }
# SWP_NOZORDER (0x0004) | SWP_NOACTIVATE (0x0010), and SWP_NOSIZE (0x0001) unless a size is given.
$flags = 0x0014
$width = $Width
$height = $Height
if ($Width -le 0 -and $Height -le 0) { $flags = 0x0015; $width = 0; $height = 0 }
[void][Shift]::SetWindowPos($handle, [IntPtr]::Zero, $rect.Left, $rect.Top - $Up, $width, $height, $flags)
Start-Sleep -Milliseconds 900
[void][Shift]::GetWindowRect($handle, [ref]$rect)
Write-Output "now at $($rect.Left),$($rect.Top) size $($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"
