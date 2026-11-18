# Independent Agent 10 Windows negative-proof tests (source-pinned, non-installed).
[CmdletBinding()]
param([Parameter(Mandatory=$true)][string]$NativeRunner)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Require([bool]$condition, [string]$message) {
  if (-not $condition) { throw "Agent10 Windows anti-spoof: $message" }
}
function Match-NativeFunction([string]$name, [string]$following) {
  $pattern = '(?ms)^function ' + [regex]::Escape($name) + '\b.*?(?=^function ' +
             [regex]::Escape($following) + '\b)'
  $m = [regex]::Match($script:qaSource, $pattern)
  Require $m.Success "cannot extract original $name function from pinned runner"
  return $m.Value
}
$script:qaSource = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $NativeRunner))
$boundSource = Match-NativeFunction 'Get-InstalledBoundDenialAlerts' 'Wait-InstalledBoundDenial'
$waitSource = Match-NativeFunction 'Wait-InstalledBoundDenial' 'Get-InstalledAngleRejectionAlerts'
$hostSource = Match-NativeFunction 'Get-InstalledHostDenial' 'Get-InstalledReferenceDenials'
Require ($boundSource.Contains('[regex]::Escape($localizedText)')) 'native alert must match exact localized text'
Require ($boundSource.Contains("' · request [0-9a-f]{32}$'")) 'native alert must carry exact 128-bit ID'
Require ($boundSource.Contains('IsOffscreen')) 'offscreen alerts must not be certified'
Require ($waitSource.Contains('$_ -cnotin $prior')) 'preexisting native alert must not be recycled'
Require ($waitSource.Contains('$fresh.Count -eq 1')) 'must accept exactly one new native alert'
Require ($hostSource.Contains('$log.Seek($beforeOffset')) 'host stderr must be read starting at click-time offset'
Require ($hostSource.Contains('$markers.Count -ne 1')) 'host stderr needs exactly one typed denial'
Require ($hostSource.Contains('$markers[0] -cne $native.Groups[1].Value')) 'backend/native IDs must match'
Invoke-Expression $hostSource

$root = Join-Path ([IO.Path]::GetTempPath()) ('agent10-denial-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($root) | Out-Null
$script:installedDenialLog = Join-Path $root 'installed-host-stderr.txt'
$label = 'The angle response is invalid or its Study evidence changed.'
$code = 'INVALID_STUDY_ANGLES'
$idA = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
$idB = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'
$boundA = $label + ' · request ' + $idA
$boundB = $label + ' · request ' + $idB
$nl = [Environment]::NewLine
function Write-Denials([string[]]$lines) {
  [IO.File]::WriteAllText($script:installedDenialLog, ($lines -join $script:nl) + $script:nl)
}
function Add-Denial([string]$line) {
  [IO.File]::AppendAllText($script:installedDenialLog, $line + $script:nl)
}
function Reject([long]$offset, [string]$wantedCode, [string]$alert, [string]$text, [string]$case) {
  $actual = Get-InstalledHostDenial $offset $wantedCode $alert $text
  Require ([string]::IsNullOrEmpty($actual)) "FAIL OPEN: $case returned backend ID '$actual'"
}
function Accept([long]$offset, [string]$wantedCode, [string]$alert, [string]$text, [string]$expected, [string]$case) {
  $actual = Get-InstalledHostDenial $offset $wantedCode $alert $text
  Require ($actual -ceq $expected) "FAIL CLOSED unexpectedly: $case expected $expected got '$actual'"
}
$tests = 0
try {
  Write-Denials @("[scriptotar-native-denial] code=$code request_id=$idA")
  $offset = [long](Get-Item -LiteralPath $script:installedDenialLog).Length
  Reject $offset $code $boundA $label 'stale host marker before clicked request'
  $tests++

  Add-Denial "[scriptotar-native-denial] code=$code request_id=$idB"
  Reject $offset $code $boundA $label 'unrelated fresh backend denial'
  $tests++
  Reject $offset $code ("Unrelated angle invalid · request " + $idB) $label 'unrelated fresh UIA alert'
  $tests++
  Reject $offset $code $label $label 'semantic text without nonce'
  $tests++
  Reject $offset $code $boundB 'Some other error' 'wrong error family/localized text'
  $tests++

  Accept $offset $code $boundB $label $idB 'one matching typed native denial'
  $tests++

  Add-Denial "[scriptotar-native-denial] code=$code request_id=$idB"
  Reject $offset $code $boundB $label 'duplicate host denial markers'
  $tests++

  Write-Denials @("[scriptotar-native-denial] code=INVALID_MEDIA_URL request_id=$idA")
  Reject 0 $code $boundA $label 'wrong backend typed error code'
  $tests++

  Write-Denials @("[scriptotar-native-denial] code=$code request_id=$idA")
  Reject 0 $code ("$label · request $idB") $label 'unbound UIA/backend IDs'
  $tests++

  Write-Denials @("[scriptotar-native-denial] code=$code request_id=$idA")
  Reject ([long]100000000) $code $boundA $label 'truncated or rotated stderr log'
  $tests++

  Write-Host "Agent 10 Windows source-pinned anti-spoof contract: $tests/$tests PASS."
  Write-Host 'Scope: real PowerShell helper under test; synthetic evidence, NOT installed UIA/Tauri 16/16.'
} finally {
  Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
