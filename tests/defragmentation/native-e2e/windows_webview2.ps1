# Installed NSIS / WebView2 native UI Automation probe.
# This does not use Chromium/Vite, frontend mocks, or backend-only IPC.
# Unimplemented scenarios are NOT TESTED and deliberately block certification.
param(
  [Parameter(Mandatory=$true)][string]$Installer,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [Parameter(Mandatory=$true)][string]$RunUrl,
  [Parameter(Mandatory=$true)][string]$EvidenceDir
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $EvidenceDir | Out-Null
$caseNames = @(
  'installed_webview', 'create_switch_projects', 'create_study',
  'edit_save_duplicate_candidate', 'quit_relaunch_persistence',
  'wrong_project_ui_isolation', 'wrong_project_ipc_denial',
  'multi_source_transcript', 'source_revocation', 'error_recovery',
  'keyboard_custom_controls', 'arabic_rtl', 'light_dark_visual'
)
$cases = [ordered]@{}
foreach ($name in $caseNames) { $cases[$name] = @{ status = 'NOT TESTED'; reason = 'Not executed' } }
$report = [ordered]@{
  scope = 'native-installed'
  platform = 'Windows WebView2 / UI Automation'
  source_sha = $SourceSha
  run_url = $RunUrl
  utc = [DateTimeOffset]::UtcNow.ToString('o')
  package = (Resolve-Path $Installer).Path
  package_sha256 = (Get-FileHash -Algorithm SHA256 $Installer).Hash.ToLowerInvariant()
  machine = @{
    os = [Environment]::OSVersion.VersionString
    cpu = $env:PROCESSOR_IDENTIFIER
    ram_kib = $null
  }
  native_ui_probe = $null
  cases = $cases
  verdict = 'NOT TESTED'
}
$installDir = Join-Path $env:RUNNER_TEMP ('scriptotar-native-' + $SourceSha.Substring(0,8))
$dataDir = Join-Path $env:RUNNER_TEMP ('scriptotar-native-data-' + $SourceSha.Substring(0,8))
$proc = $null
try {
  if ($SourceSha -notmatch '^[a-f0-9]{40}$' -or $RunUrl -notmatch '^https://github.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*$') {
    throw 'Invalid exact-commit evidence metadata'
  }
  Remove-Item -Force -Recurse $installDir,$dataDir -ErrorAction SilentlyContinue
  $installed = Start-Process -FilePath $Installer -ArgumentList @('/S', "/D=$installDir") -PassThru -Wait
  if ($installed.ExitCode -ne 0) { throw "NSIS install exited $($installed.ExitCode)" }
  $app = Get-ChildItem $installDir -Recurse -Filter scriptotar-desktop.exe | Select-Object -First 1
  if (-not $app) { throw 'Installed scriptotar-desktop.exe not found' }
  $report.executable = $app.FullName
  $report.executable_sha256 = (Get-FileHash -Algorithm SHA256 $app.FullName).Hash.ToLowerInvariant()
  New-Item -ItemType Directory -Force $dataDir | Out-Null
  $env:SCRIPTOTAR_DATA_DIR = $dataDir
  $proc = Start-Process -FilePath $app.FullName -PassThru
  Add-Type -AssemblyName UIAutomationClient
  Add-Type -AssemblyName UIAutomationTypes
  Add-Type -AssemblyName System.Drawing

  $root = $null
  for ($i = 0; $i -lt 35; $i++) {
    Start-Sleep -Seconds 1
    $proc.Refresh()
    if ($proc.HasExited) { throw "Installed GUI exited $($proc.ExitCode)" }
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) { continue }
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)
      if ($root) { break }
    } catch { }
  }
  if (-not $root) {
    $cases.installed_webview = @{ status='NOT TESTED'; reason='No usable interactive desktop/UIA main window on Windows runner' }
  } else {
    $tree = [System.Windows.Automation.TreeScope]::Descendants
    $condition = [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Document
    )
    $document = $root.FindFirst($tree, $condition)
    $buttonCondition = [System.Windows.Automation.AndCondition]::new(
      [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button),
      [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty, 'Studio')
    )
    $studioButton = $root.FindFirst($tree, $buttonCondition)
    if (-not $document -or -not $studioButton) {
      $cases.installed_webview = @{
        status='NOT TESTED'
        reason='Installed shell present, but WebView2 UIA Document + Studio interactive button not exposed'
      }
    } else {
      $invoke = $studioButton.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      Start-Sleep -Milliseconds 750
      $headingCondition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
          [System.Windows.Automation.ControlType]::Text),
        [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, 'Studio')
      )
      # WebView2 may publish headings as Text, Header, or Group.
      $heading = $root.FindFirst($tree, $headingCondition)
      if (-not $heading) {
        $cases.installed_webview = @{ status='FAIL'; reason='Studio button invoked but expected Studio heading not accessible' }
      } else {
        $cases.installed_webview = @{
          status='PASS'
          reason='Installed NSIS binary exposed a WebView2 UIA document and Studio button was invoked'
        }
        $report.native_ui_probe = 'installed-webview-automation'
        $bounds = $root.Current.BoundingRectangle
        if ($bounds.Width -gt 0 -and $bounds.Height -gt 0) {
          $bitmap = New-Object System.Drawing.Bitmap([int]$bounds.Width,[int]$bounds.Height)
          $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
          try {
            $graphics.CopyFromScreen(
              [int]$bounds.X,[int]$bounds.Y,0,0,
              $bitmap.Size
            )
            $bitmap.Save((Join-Path $EvidenceDir 'installed-webview2.png'))
          } finally { $graphics.Dispose(); $bitmap.Dispose() }
        }
      }
    }
  }
} catch {
  $message = $_.Exception.Message
  $cases.installed_webview = @{ status='FAIL'; reason="Native probe failed: $message" }
  $report.error = $message
} finally {
  if ($proc -and -not $proc.HasExited) {
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  }
  $uninstaller = Get-ChildItem $installDir -Recurse -Filter uninstall.exe -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($uninstaller) {
    $cleanup = Start-Process -FilePath $uninstaller.FullName -ArgumentList '/S' -PassThru -Wait
    $report.uninstall_exit_code = $cleanup.ExitCode
  }
}
$states = @($cases.Values | ForEach-Object { $_.status })
if ($states -contains 'FAIL') { $report.verdict = 'FAIL' }
elseif ($states -contains 'NOT TESTED') { $report.verdict = 'NOT TESTED' }
else { $report.verdict = 'PASS' }
$report | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 (Join-Path $EvidenceDir 'result.json')
Write-Host ($report | ConvertTo-Json -Depth 8)
# Partial Windows UIA evidence is not release certification.
if ($report.verdict -ne 'PASS') { exit 1 }
