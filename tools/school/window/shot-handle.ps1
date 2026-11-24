# Save the picture of one window, by its handle, to a PNG file.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File shot-handle.ps1 -Handle 22679598 -Out window.png
param(
    [Parameter(Mandatory = $true)][long]$Handle,
    [Parameter(Mandatory = $true)][string]$Out
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class HandleShot {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
}
'@
$window = [IntPtr]$Handle
$rect = New-Object HandleShot+RECT
[void][HandleShot]::GetWindowRect($window, [ref]$rect)
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top
$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$dc = $graphics.GetHdc()
$captured = [HandleShot]::PrintWindow($window, $dc, 2)
$graphics.ReleaseHdc($dc)
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
"Window: $width x $height; captured: $captured; saved: $Out"
