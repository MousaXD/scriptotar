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
    child_windows = @()
    child_webview_provider_nodes = @()
  }
  if ($root) {
    if ($appProcess.MainWindowHandle -ne [IntPtr]::Zero) {
      foreach ($hwnd in [NativeWebViewWindow]::Children($appProcess.MainWindowHandle)) {
        $probe.child_windows += @{
          hwnd = $hwnd.ToInt64()
          class_name = [NativeWebViewWindow]::ClassName($hwnd)
          process_id = [NativeWebViewWindow]::ProcessId($hwnd)
        }
      }
    }
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    # Tauri/WRY's parent Pane can omit the WebView2 UIA fragment.
    # Record bounded accessibility descendants starting at trusted child HWNDs.
    $checkedHandles = 0
    foreach ($hwnd in [NativeWebViewWindow]::Children($appProcess.MainWindowHandle)) {
      if ($checkedHandles -ge 20 -or $probe.child_webview_provider_nodes.Count -ge 240) { break }
      $className = [NativeWebViewWindow]::ClassName($hwnd)
      if ($className -notmatch 'Chrome_RenderWidgetHostHWND|Chrome_WidgetWin_0|Chrome_WidgetWin_1') { continue }
      $checkedHandles++
      try {
        $providerRoot = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
        if (-not $providerRoot) { continue }
        $webQueue = [System.Collections.Generic.Queue[object]]::new()
        $webQueue.Enqueue(@($providerRoot,0))
        while ($webQueue.Count -gt 0 -and $probe.child_webview_provider_nodes.Count -lt 240) {
          $entry = $webQueue.Dequeue()
          $element = $entry[0]
          $depth = [int]$entry[1]
          try {
            $probe.child_webview_provider_nodes += @{
              hwnd = $hwnd.ToInt64()
              host_class = $className
              depth = $depth
              control_type = $element.Current.ControlType.ProgrammaticName
              name = [string]$element.Current.Name
            }
            if ($depth -lt 8) {
              $child = $walker.GetFirstChild($element)
              while ($child -and $webQueue.Count -lt 180) {
                $webQueue.Enqueue(@($child,($depth+1)))
                $child = $walker.GetNextSibling($child)
              }
            }
          } catch {
            $probe.child_webview_provider_nodes += @{
              hwnd = $hwnd.ToInt64()
              depth = $depth
              error = $_.Exception.GetType().Name
            }
          }
        }
      } catch {
        $probe.child_webview_provider_nodes += @{
          hwnd = $hwnd.ToInt64()
          host_class = $className
          error = $_.Exception.Message
        }
      }
    }
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

# UI Automation on a WRY_WEBVIEW host Pane may be an orphaned fragment:
# WebView2's document provider can live on a nested render-widget HWND.
# Enumerate ONLY HWND descendants of this installed application's main window.
# Never search arbitrary desktop windows or accept unrelated browser content.
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeWebViewWindow {
    public delegate bool EnumChildProc(IntPtr hwnd, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr parent, EnumChildProc callback, IntPtr data);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, StringBuilder buffer, int length);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
    public static List<IntPtr> Children(IntPtr parent) {
        var found = new List<IntPtr>();
        EnumChildProc callback = (hwnd, data) => { if (found.Count < 200) found.Add(hwnd); return found.Count < 200; };
        EnumChildWindows(parent, callback, IntPtr.Zero);
        return found;
    }
    public static string ClassName(IntPtr hwnd) {
        var name = new StringBuilder(256);
        GetClassName(hwnd, name, name.Capacity);
        return name.ToString();
    }
    public static uint ProcessId(IntPtr hwnd) {
        uint pid;
        GetWindowThreadProcessId(hwnd, out pid);
        return pid;
    }
}
'@

function Find-InstalledDocumentAndStudio([object]$process) {
  $scope = [System.Windows.Automation.TreeScope]::Descendants
  $docCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Document
  )
  $studioCondition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Button),
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::NameProperty, 'Studio')
  )
  $handles = @($process.MainWindowHandle)
  $handles += @([NativeWebViewWindow]::Children($process.MainWindowHandle) |
    Where-Object { [NativeWebViewWindow]::ClassName($_) -match 'Chrome_|WRY_|WebView' })
  $foundDocument = $null
  $foundStudio = $null
  $providerHandle = $null
  foreach ($handle in $handles) {
    if ($handle -eq [IntPtr]::Zero) { continue }
    try {
      $candidate = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
      if (-not $candidate) { continue }
      # Include the root node, as an orphaned render HWND may itself be a
      # Document provider, unlike the parent WRY_WEBVIEW Pane.
      $scopeBoth = [System.Windows.Automation.TreeScope]::Subtree
      $document = $candidate.FindFirst($scopeBoth, $docCondition)
      if (-not $document) { continue }
      $studioButton = $candidate.FindFirst($scopeBoth, $studioCondition)
      if ($document -and $studioButton) {
        return @{ document=$document; studio=$studioButton; hwnd=$handle }
      }
      $foundDocument = $document
      $providerHandle = $handle
    } catch {
      # UIA providers may disconnect during WebView2 initialization.
    }
  }
  return @{ document=$foundDocument; studio=$foundStudio; hwnd=$providerHandle }
}

$testTemp = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$installDir = Join-Path $testTemp ('scriptotar-native-' + $SourceSha.Substring(0,8))
$dataDir = Join-Path $testTemp ('scriptotar-native-data-' + $SourceSha.Substring(0,8))
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
  $probe = $null
  $readiness = [ordered]@{
    main_window_seen = $false
    web_document_seen = $false
    studio_control_seen = $false
    poll_count = 0
    source_process_id = $proc.Id
    foreground_requested = $false
    elapsed_seconds = 0
  }
  $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
  # The old probe stopped at the first native shell HWND. On this runner
  # that frame was white and only contained a WRY_WEBVIEW Pane. Wait for
  # the actual document and controls, checking nested WebView2 HWNDs.
  while ($stopwatch.Elapsed.TotalSeconds -lt 90) {
    Start-Sleep -Seconds 2
    $readiness.poll_count++
    $proc.Refresh()
    if ($proc.HasExited) { throw "Installed GUI exited $($proc.ExitCode)" }
    if ($proc.MainWindowHandle -eq [IntPtr]::Zero) { continue }
    $readiness.main_window_seen = $true
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)
      if (-not $root) { continue }
      $readiness.foreground_requested = [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle)
      $probe = Find-InstalledDocumentAndStudio $proc
      if ($probe.document) { $readiness.web_document_seen = $true }
      if ($probe.studio) { $readiness.studio_control_seen = $true }
      if ($probe.document -and $probe.studio) { break }
    } catch {
      $readiness['last_provider_exception'] = $_.Exception.Message
    }
  }
  $stopwatch.Stop()
  $readiness.elapsed_seconds = [Math]::Round($stopwatch.Elapsed.TotalSeconds,1)
  $report.readiness = $readiness
  # WebView2 may have failed to spawn/render even though the Tauri shell exists.
  # Record child process ancestry, not command lines or unrelated user data.
  $report.webview2_processes = @(
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" -ErrorAction SilentlyContinue |
      Where-Object { $_.SessionId -eq $proc.SessionId } |
      Select-Object ProcessId,ParentProcessId,SessionId
  )
  $report.webview2_process_count_in_session = @($report.webview2_processes).Count
  Write-UiaDiagnostics $root (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
  $report.uia_diagnostics = 'uia-diagnostics.json'
  Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'unverified-window.png')
  if (-not $root) {
    $cases.installed_webview = @{ status='NOT TESTED'; reason='No usable UIA main window for the installed process; see uia-diagnostics.json' }
  } elseif (-not $probe -or -not $probe.document -or -not $probe.studio) {
    $cases.installed_webview = @{
      status='NOT TESTED'
      reason='After 90s, installed shell had no confirmed WebView2 Document + Studio button; nested HWNDs and readiness logged in uia-diagnostics.json/result.json'
    }
  } else {
    try {
      $invoke = $probe.studio.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      Start-Sleep -Milliseconds 900
      # Use the SAME WebView2 provider subtree as the discovered button.
      $scope = [System.Windows.Automation.TreeScope]::Subtree
      $headingCondition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
          [System.Windows.Automation.ControlType]::Text),
        [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, 'Studio')
      )
      $provider = [System.Windows.Automation.AutomationElement]::FromHandle($probe.hwnd)
      $heading = $provider.FindFirst($scope, $headingCondition)
      if (-not $heading) {
        $cases.installed_webview = @{ status='FAIL'; reason='Studio control invoked but Studio heading not exposed within the installed document' }
      } else {
        $cases.installed_webview = @{
          status='PASS'
          reason='Installed NSIS WebView2 Document was found under a descendant HWND and Studio invoked via UIA'
        }
        $report.native_ui_probe = 'installed-webview-automation'
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'installed-webview2.png')
      }
    } catch {
      $cases.installed_webview = @{ status='FAIL'; reason="Installed GUI action failed: $($_.Exception.Message)" }
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
