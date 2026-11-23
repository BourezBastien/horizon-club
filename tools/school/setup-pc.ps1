# Horizon Club: what each student's PC needs, once, with an administrator's rights (by hand, or
# through a Group Policy start script). Two things:
#
#   - a shortcut on the Public desktop, so every account of the PC finds the club's launcher;
#   - firewall rules, so other students can join the servers hosted from this PC, and see the
#     worlds opened to the LAN.
#
#   powershell -ExecutionPolicy Bypass -File setup-pc.ps1 -Target "\\SERVEUR\Horizon\HorizonClub.exe"
#
# `-Target` is the stable entry point at the root of the club's share (docs/school-edition).
# The rules let in the hosted servers' ports only (TCP 25565 to 25575 by default: the same
# numbers as `first_port` and `last_port` in club.toml), and the LAN discovery datagrams
# (UDP 4445), from the local network only. A world opened to the LAN from the pause menu takes
# a port Windows picks at random: without a rule for the game's own program, which has a
# different path for every student, other students cannot join it. Hosting from the launcher's
# Servers page has no such limit.
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)][string]$Target,
    [string]$Name = 'Horizon Club',
    [int]$FirstPort = 25565,
    [int]$LastPort = 25575,
    [int]$DiscoveryPort = 4445
)

$ErrorActionPreference = 'Stop'

$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
$administrators = [System.Security.Principal.WindowsBuiltInRole]::Administrator
if (-not $principal.IsInRole($administrators)) {
    throw 'Run this script as an administrator.'
}
if ($LastPort -lt $FirstPort) {
    throw '-LastPort must not be smaller than -FirstPort.'
}

# The shortcut, written by the shell itself: a hand-made .lnk file has no target list, and
# Explorer then shows an empty target and starts nothing.
$desktop = [Environment]::GetFolderPath('CommonDesktopDirectory')
$link = Join-Path $desktop ($Name + '.lnk')
if ($PSCmdlet.ShouldProcess($link, "Create a shortcut to $Target")) {
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($link)
    $shortcut.TargetPath = $Target
    $shortcut.Save()
}

# One rule per protocol: a port range on TCP for the hosted servers, the fixed UDP port for the
# LAN discovery. Both are replaced, so a changed port range takes effect when this runs again.
$rules = @(
    @{ Name = "$Name - hosted servers"; Protocol = 'TCP'; Port = "$FirstPort-$LastPort" },
    @{ Name = "$Name - LAN discovery"; Protocol = 'UDP'; Port = "$DiscoveryPort" }
)
foreach ($rule in $rules) {
    if (-not $PSCmdlet.ShouldProcess($rule.Name, 'Create the firewall rule')) {
        continue
    }
    Get-NetFirewallRule -DisplayName $rule.Name -ErrorAction SilentlyContinue | Remove-NetFirewallRule
    New-NetFirewallRule `
        -DisplayName $rule.Name `
        -Group $Name `
        -Direction Inbound `
        -Action Allow `
        -Profile Domain, Private `
        -Protocol $rule.Protocol `
        -LocalPort $rule.Port `
        -RemoteAddress LocalSubnet | Out-Null
}

Write-Output "Shortcut: $link"
Write-Output "Firewall: TCP $FirstPort-$LastPort and UDP $DiscoveryPort, inbound, local network only."
