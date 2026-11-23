# Horizon Club: the folders and the permissions of the club's share. Run once, with an
# administrator's rights, on the computer that hosts the share, on the folder the share shows:
#
#   powershell -ExecutionPolicy Bypass -File setup-share.ps1 -Root D:\Horizon `
#       -Students "COLLEGE\Eleves" -Supervisor "COLLEGE\jdupont"
#
# The share itself (its name, and who may open it) is the server's own tool's job: give the
# students' group and the supervisor access to it. This script makes the four folders the
# launcher looks for and sets who writes where:
#
#   app\     the launcher, version by version            supervisor writes, students read
#   core\    the game builds and their files             supervisor writes, students read
#   online\  the servers students host, announced there  everyone writes
#   admin\   the supervisor's tools                      supervisor only
#
# `-Students` takes several groups or accounts. `-WhatIf` shows what would change.
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)][string]$Root,
    [Parameter(Mandatory = $true)][string[]]$Students,
    [Parameter(Mandatory = $true)][string]$Supervisor
)

$ErrorActionPreference = 'Stop'

function ConvertTo-Sid {
    param([string]$Account)
    (New-Object System.Security.Principal.NTAccount($Account)).Translate([System.Security.Principal.SecurityIdentifier])
}

# The well-known accounts, by their identifier: their names change with the system's language.
$system = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-18')
$administrators = New-Object System.Security.Principal.SecurityIdentifier('S-1-5-32-544')
# Kept in variables of their own name: PowerShell ignores case in variable names, and the
# parameters above are typed as text, which would turn an identifier back into a string.
$supervisorSid = ConvertTo-Sid $Supervisor
$studentSids = @($Students | ForEach-Object { ConvertTo-Sid $_ })

# The rights of one folder: its own rules, none inherited from above.
function Set-FolderRights {
    param(
        [string]$Path,
        [System.Security.Principal.IdentityReference[]]$Full,
        [System.Security.Principal.IdentityReference[]]$Modify = @(),
        [System.Security.Principal.IdentityReference[]]$Read = @()
    )
    if (-not $PSCmdlet.ShouldProcess($Path, 'Set the permissions')) {
        return
    }
    $rights = New-Object System.Security.AccessControl.DirectorySecurity
    $rights.SetAccessRuleProtection($true, $false)
    $inherit = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
    $none = [System.Security.AccessControl.PropagationFlags]::None
    $allow = [System.Security.AccessControl.AccessControlType]::Allow
    foreach ($set in @(
            @($Full, [System.Security.AccessControl.FileSystemRights]::FullControl),
            @($Modify, [System.Security.AccessControl.FileSystemRights]::Modify),
            @($Read, [System.Security.AccessControl.FileSystemRights]::ReadAndExecute)
        )) {
        foreach ($who in $set[0]) {
            $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($who, $set[1], $inherit, $none, $allow)
            $rights.AddAccessRule($rule)
        }
    }
    Set-Acl -LiteralPath $Path -AclObject $rights
}

$keepers = @($system, $administrators, $supervisorSid)
$layout = @(
    @{ Folder = ''; Full = $keepers; Modify = @(); Read = $studentSids },
    @{ Folder = 'app'; Full = $keepers; Modify = @(); Read = $studentSids },
    @{ Folder = 'core'; Full = $keepers; Modify = @(); Read = $studentSids },
    @{ Folder = 'online'; Full = @($system, $administrators); Modify = ($studentSids + $supervisorSid); Read = @() },
    @{ Folder = 'admin'; Full = $keepers; Modify = @(); Read = @() }
)
foreach ($folder in $layout) {
    $path = if ($folder.Folder) { Join-Path $Root $folder.Folder } else { $Root }
    if (-not (Test-Path -LiteralPath $path) -and $PSCmdlet.ShouldProcess($path, 'Create the folder')) {
        New-Item -ItemType Directory -Path $path | Out-Null
    }
    Set-FolderRights -Path $path -Full $folder.Full -Modify $folder.Modify -Read $folder.Read
}

Write-Output "Ready: $Root (club.toml, app\, core\, online\, admin\)."
