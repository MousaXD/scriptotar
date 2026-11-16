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
  'keyboard_custom_controls', 'arabic_locale_control', 'theme_mode_control',
  'arabic_rtl', 'light_dark_visual'
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
function Write-UiaDiagnostics([object]$root, [string]$path, [object]$appProcess) {
  # RawView includes provider objects that ControlView and Document queries
  # may omit. This is diagnostic evidence, NOT an acceptance bypass.
  $probe = [ordered]@{
    runner_interactive = [Environment]::UserInteractive
    runner_session_id = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    app_session_id = $appProcess.SessionId
    main_window_handle = [string]$appProcess.MainWindowHandle
    nodes = @()
  }
  if ($root) {
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $pending = [System.Collections.Generic.Queue[object]]::new()
    $pending.Enqueue(@($root,0))
    while ($pending.Count -gt 0 -and $probe.nodes.Count -lt 180) {
      $item = $pending.Dequeue()
      $node = $item[0]
      $depth = [int]$item[1]
      try {
        $probe.nodes += @{ depth=$depth; control_type=$node.Current.ControlType.ProgrammaticName;
          name=[string]$node.Current.Name; class_name=[string]$node.Current.ClassName }
        if ($depth -lt 6) {
          $child = $walker.GetFirstChild($node)
          while ($child) {
            $pending.Enqueue(@($child,($depth+1)))
            $child = $walker.GetNextSibling($child)
          }
        }
      } catch {
        $probe.nodes += @{ depth=$depth; error=$_.Exception.GetType().Name }
      }
    }
  }
  $probe | ConvertTo-Json -Depth 12 | Set-Content -Encoding utf8 $path
}
function Save-UnverifiedWindow([object]$root, [string]$path) {
  if (-not $root) { return }
  try {
    $bounds = $root.Current.BoundingRectangle
    if ($bounds.Width -le 0 -or $bounds.Height -le 0) { return }
    $bitmap = [System.Drawing.Bitmap]::new([int]$bounds.Width,[int]$bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
      $graphics.CopyFromScreen([int]$bounds.X,[int]$bounds.Y,0,0,$bitmap.Size)
      $bitmap.Save($path)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
  } catch {
    Write-Warning ("Installed shell screenshot capture failed: " + $_.Exception.Message)
  }
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
  Write-UiaDiagnostics $root (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
  $report.uia_diagnostics = 'uia-diagnostics.json'
  Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'unverified-window.png')
  if (-not $root) {
    $cases.installed_webview = @{ status='NOT TESTED'; reason='No usable interactive desktop/UIA main window on Windows runner; see uia-diagnostics.json' }
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
        reason='Installed shell present, but WebView2 UIA Document + Studio button not exposed; inspect uia-diagnostics.json and unverified-window.png'
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
