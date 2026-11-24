# Post a click, a mouse move or a mouse wheel straight to a window's message queue.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File post.ps1 -Process HorizonClubLauncher -Action click -At "307,443"
#   powershell -NoProfile -ExecutionPolicy Bypass -File post.ps1 -Process HorizonClubLauncher -Action wheel -At "778,450" -Times 6
#   powershell -NoProfile -ExecutionPolicy Bypass -File post.ps1 -Process HorizonClubLauncher -Action move -At "1155,18"
#
# -Process  the name of the process whose main window receives it
# -Action   click, move (the pointer is only announced there: what is under it shows its hover,
#           while the person's own pointer stays where it is), or wheel (it rolls down)
# -At       a point in pixels from the window's top-left corner, as measured on a picture of the
#           window taken by shot.ps1. Write it "x,y", quoted: a bare 307,443 would be read as
#           the number 307443.
# -Times    how many notches the wheel rolls (default 5)
# -Id       the process itself, when several of one name run (a test's beside the player's own)
#
# The launcher's window (GPUI) and the game's (winit) take these messages, where the global
# mouse functions (SetCursorPos, mouse_event, SendKeys) reach nothing from an agent's shell.
# One script, one action: a click that opens a page needs a new shot.ps1 to find the next point.
# A window that is behind others, or scrolled, is not moved or raised.
[CmdletBinding()]
param(
    [string]$Process = 'HorizonClubLauncher',
    [Parameter(Mandatory = $true)][ValidateSet('wheel', 'click', 'move')][string]$Action,
    [string]$At = '600,500',
    [int]$Times = 5,
    [int]$Id = 0
)

$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Post {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr w, out RECT r);
  [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr w, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool PostMessage(IntPtr w, uint msg, IntPtr wp, IntPtr lp);
}
'@
[void][Post]::SetProcessDPIAware()
$found = if ($Id -gt 0) {
    Get-Process -Id $Id -ErrorAction SilentlyContinue
} else {
    Get-Process -Name $Process -ErrorAction SilentlyContinue |
        Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
}
if (-not $found -or $found.MainWindowHandle -eq 0) { throw "No window of $Process." }
$handle = $found.MainWindowHandle
$rect = New-Object Post+RECT
if (-not [Post]::GetWindowRect($handle, [ref]$rect)) { throw 'GetWindowRect failed.' }
$xy = $At.Split(',')
$point = New-Object Post+POINT
$point.X = $rect.Left + [int]$xy[0]
$point.Y = $rect.Top + [int]$xy[1]
[void][Post]::ScreenToClient($handle, [ref]$point)
$place = [IntPtr](($point.Y -shl 16) -bor ($point.X -band 0xFFFF))
if ($Action -eq 'move') {
    # WM_MOUSEMOVE alone, twice so the hover is certain to follow.
    for ($n = 0; $n -lt 2; $n++) {
        [void][Post]::PostMessage($handle, 0x0200, [IntPtr]::Zero, $place)
        Start-Sleep -Milliseconds 150
    }
} elseif ($Action -eq 'wheel') {
    # WM_MOUSEWHEEL: the high word of wParam is the wheel's step; negative rolls down. Unlike a
    # move, its point counts from the screen's corner, not the window's.
    $step = [IntPtr](-120 -shl 16)
    $onScreen = [IntPtr]((($rect.Top + [int]$xy[1]) -shl 16) -bor (($rect.Left + [int]$xy[0]) -band 0xFFFF))
    for ($n = 0; $n -lt $Times; $n++) {
        [void][Post]::PostMessage($handle, 0x0200, [IntPtr]::Zero, $place)
        [void][Post]::PostMessage($handle, 0x020A, $step, $onScreen)
        Start-Sleep -Milliseconds 80
    }
} else {
    [void][Post]::PostMessage($handle, 0x0200, [IntPtr]::Zero, $place)
    Start-Sleep -Milliseconds 150
    [void][Post]::PostMessage($handle, 0x0201, [IntPtr]1, $place)
    Start-Sleep -Milliseconds 80
    [void][Post]::PostMessage($handle, 0x0202, [IntPtr]::Zero, $place)
}
Write-Output "$Action posted to $Process at client $($point.X),$($point.Y)"
