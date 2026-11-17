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
  'installed_webview', 'create_switch_projects', 'canonical_project_persistence', 'create_study',
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
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
    public static void NativeClick(int x, int y) {
        if (!SetCursorPos(x, y)) throw new InvalidOperationException("Cannot position pointer at UIA option");
        System.Threading.Thread.Sleep(120);
        mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); // MOUSEEVENTF_LEFTDOWN
        System.Threading.Thread.Sleep(65);
        mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero); // MOUSEEVENTF_LEFTUP
    }
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

# The following helpers operate ONLY on the installed app's trusted WebView2
# render HWND; they never search another app's desktop UI.
function Find-ExactInstalledControl([IntPtr]$hwnd, [string]$name, [object]$kind, [int]$timeoutSec=12) {
  $scope = [System.Windows.Automation.TreeScope]::Subtree
  $exact = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty, $name)
  $condition = if ($kind) {
    [System.Windows.Automation.AndCondition]::new(
      $exact,
      [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $kind))
  } else { $exact }
  $deadline = [DateTime]::UtcNow.AddSeconds($timeoutSec)
  do {
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      $element = $root.FindFirst($scope, $condition)
      if ($element) { return $element }
    } catch { }
    Start-Sleep -Milliseconds 350
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed WebView2 control missing: $name (role $kind)"
}
function Click-InstalledControl([IntPtr]$hwnd, [string]$name, [object]$kind, [switch]$pointer) {
  $element = Find-ExactInstalledControl $hwnd $name $kind
  if (-not $pointer) {
    try {
      $invoke = $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      Start-Sleep -Milliseconds 550
      return
    } catch {
      # Chrome maps aria-option/list item without a usable InvokePattern.
      # A native geometry pointer action is required for such controls.
    }
  }
  $point = $element.GetClickablePoint()
  if ([Double]::IsNaN($point.X) -or [Double]::IsNaN($point.Y)) {
    throw "Invalid UIA pointer coordinates for installed control $name"
  }
  [NativeWebViewWindow]::NativeClick([int][Math]::Round($point.X), [int][Math]::Round($point.Y))
  Start-Sleep -Milliseconds 650
}
function Type-InstalledEdit([IntPtr]$hwnd, [string]$name, [string]$value) {
  $element = Find-ExactInstalledControl $hwnd $name ([System.Windows.Automation.ControlType]::Edit)
  $element.SetFocus()
  # Keyboard dispatch triggers the real Svelte input listeners. UIA SetValue
  # alone can replace a DOM value without emitting input events.
  [System.Windows.Forms.SendKeys]::SendWait('^a')
  [System.Windows.Forms.SendKeys]::SendWait($value)
  Start-Sleep -Milliseconds 650
  $actual = $element.GetCurrentPattern(
    [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
  if ($actual -ne $value) {
    throw "Installed Edit control '$name' did not receive actual user text"
  }
}
function Wait-InstalledDashboard([IntPtr]$hwnd, [string]$name) {
  # In the captured WebView2 tree the current project is twice exposed as
  # Text (combobox selected value + actual visible dashboard h1). Requiring
  # both and the New project button prevents mere chooser options from PASS.
  $nameCond = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::NameProperty, $name),
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Text))
  $deadline = [DateTime]::UtcNow.AddSeconds(18)
  do {
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      $items = $root.FindAll([System.Windows.Automation.TreeScope]::Subtree, $nameCond)
      if ($items.Count -ge 2) {
        $newProject = $root.FindFirst([System.Windows.Automation.TreeScope]::Subtree,
          [System.Windows.Automation.AndCondition]::new(
            [System.Windows.Automation.PropertyCondition]::new(
              [System.Windows.Automation.AutomationElement]::NameProperty, 'New project'),
            [System.Windows.Automation.PropertyCondition]::new(
              [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
              [System.Windows.Automation.ControlType]::Button)))
        if ($newProject) { return }
      }
    } catch { }
    Start-Sleep -Milliseconds 400
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed dashboard failed to activate project $name"
}
function Find-InstalledPopupOption([IntPtr]$hwnd, [string]$name, [int]$timeoutSec=7) {
  # A selected Study is also exposed as a normal Text node in "Open study".
  # Match the REAL open popup's option role, not the first equal-name Text
  # anywhere in the current installed document.
  $matchName = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty, $name)
  $deadline = [DateTime]::UtcNow.AddSeconds($timeoutSec)
  do {
    try {
      $provider = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      $matches = $provider.FindAll(
        [System.Windows.Automation.TreeScope]::Subtree, $matchName)
      foreach ($element in $matches) {
        $role = $element.Current.ControlType.ProgrammaticName
        $rect = $element.Current.BoundingRectangle
        if ($role -in @('ControlType.ListItem', 'ControlType.Button') -and
            $rect.Width -gt 2 -and $rect.Height -gt 2) {
          return $element
        }
      }
    } catch { }
    Start-Sleep -Milliseconds 250
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed popup option not found with actionable UIA role: $name"
}
function Choose-InstalledOption([IntPtr]$hwnd, [string]$label, [string]$name) {
  Click-InstalledControl $hwnd $label ([System.Windows.Automation.ControlType]::ComboBox) -pointer
  $option = Find-InstalledPopupOption $hwnd $name 7
  $actualRole = $option.Current.ControlType.ProgrammaticName
  if ($actualRole -notin @('ControlType.ListItem', 'ControlType.Button')) {
    throw "Project option has unexpected UIA role $actualRole (expected native list item/button)"
  }
  $provider = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
  $optionType = if ($actualRole -eq 'ControlType.ListItem') {
    [System.Windows.Automation.ControlType]::ListItem
  } else { [System.Windows.Automation.ControlType]::Button }
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $optionType)
  $allOptions = $provider.FindAll([System.Windows.Automation.TreeScope]::Subtree, $condition)
  $rect = $option.Current.BoundingRectangle
  $ordered = @($allOptions | Where-Object {
    ($_.Current.Name -eq 'Inbox' -or $_.Current.Name -like 'NativeQA-*') -and
    $_.Current.BoundingRectangle.Width -gt 2 -and
    $_.Current.BoundingRectangle.Height -gt 2 -and
    [Math]::Abs($_.Current.BoundingRectangle.X - $rect.X) -lt 20
  } | Sort-Object { $_.Current.BoundingRectangle.Y })
  $labels = @($ordered | ForEach-Object { [string]$_.Current.Name })
  $index = [Array]::IndexOf($labels, $name)
  if ($index -lt 0 -or $labels.Count -gt 30) {
    throw "Could not establish exact UIA option order for $name; seen=$($labels -join ',')"
  }
  # The Svelte custom Select moves focus to its listbox after opening and
  # handles Home/ArrowDown/Enter. Use keyboard events on that real popup,
  # with the option index grounded in UIA items, never a guessed position.
  Start-Sleep -Milliseconds 400
  [System.Windows.Forms.SendKeys]::SendWait('{HOME}')
  for ($i = 0; $i -lt $index; $i++) {
    [System.Windows.Forms.SendKeys]::SendWait('{DOWN}')
  }
  [System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
}
function Choose-InstalledProject([IntPtr]$hwnd, [string]$name) {
  Choose-InstalledOption $hwnd 'PROJECT' $name
  Wait-InstalledDashboard $hwnd $name
}
function Read-InstalledEdit([IntPtr]$hwnd, [string]$name) {
  $field = Find-ExactInstalledControl $hwnd $name ([System.Windows.Automation.ControlType]::Edit)
  return [string]$field.GetCurrentPattern(
    [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
}
function Verify-InstalledCanonicalDb([string]$kind, [string]$first, [string]$second, [string]$third) {
  # Windows CI has Python 3.12, and this imports only standard library code.
  # The canonical probe opens installed app SQLite in mode=ro and never
  # changes user state, shortcuts project selection or fabricates GUI PASS.
  $qaRoot = (Resolve-Path 'tests/defragmentation/native-e2e').Path
  $probe = @'
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from linux_webkit import verify_canonical_projects, verify_canonical_saved_draft
kind, root, a, b, c = sys.argv[2:7]
if kind == "projects":
    print(verify_canonical_projects(Path(root), (a, b)))
elif kind == "draft":
    print(verify_canonical_saved_draft(Path(root), a, b, c))
else:
    raise ValueError("Unsupported canonical installed DB probe")
'@
  $result = & python -c $probe $qaRoot $kind $dataDir $first $second $third 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw "Installed canonical SQLite $kind probe failed: $($result | Out-String)"
  }
  return [string]($result | Out-String).Trim()
}
function Wait-InstalledCanonicalDb([string]$kind, [string]$first, [string]$second, [string]$third) {
  # The real UI can complete an auto-save shortly after a native key event.
  # Poll the authoritative SQLite snapshot instead of racing an inaccessible
  # status label or interpreting a temporarily disabled save button as failure.
  $lastError = 'Not checked'
  for ($attempt=0; $attempt -lt 24; $attempt++) {
    try {
      return Verify-InstalledCanonicalDb $kind $first $second $third
    } catch {
      $lastError = $_.Exception.Message
      Start-Sleep -Milliseconds 350
    }
  }
  throw "Installed canonical SQLite $kind never reached the expected state: $lastError"
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
  Add-Type -AssemblyName System.Windows.Forms

  $root = $null
  $probe = $null
  $readiness = [ordered]@{
    main_window_seen = $false
    web_document_seen = $false
    studio_control_seen = $false
    dev_server_error_seen = $false
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
      # Exact previously captured installed browser failure uses this app-
      # owned UIA pane name even if Document does not expose the error text.
      $networkErrorPane = $root.FindFirst(
        [System.Windows.Automation.TreeScope]::Subtree,
        [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty,
          'localhost - Network error - Web content'))
      if ($networkErrorPane) {
        $readiness.dev_server_error_seen = $true
        break
      }
      if ($probe.document) {
        $readiness.web_document_seen = $true
        # A WebView2 Document can be the built-in browser network-error
        # document. Fail early instead of polling 90s and misreporting a
        # missing UIA provider when the installed binary points at devUrl.
        $connectionRefused = [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, 'ERR_CONNECTION_REFUSED'
        )
        if ($probe.document.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree, $connectionRefused)) {
          $readiness.dev_server_error_seen = $true
          break
        }
      }
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
    # When a genuine WebView2 document exposes the dev server error page,
    # this is a failed installed product/installer, not missing UIA support.
    $hasDevServerError = $false
    if ($probe -and $probe.document) {
      try {
        $networkError = [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, 'ERR_CONNECTION_REFUSED'
        )
        $hasDevServerError = [bool]$probe.document.FindFirst(
          [System.Windows.Automation.TreeScope]::Subtree, $networkError
        )
      } catch {
        $report['network_error_probe_warning'] = $_.Exception.Message
      }
    }
    if ($readiness.dev_server_error_seen -or $hasDevServerError) {
      $cases.installed_webview = @{
        status='FAIL'
        reason='Installed NSIS binary rendered localhost ERR_CONNECTION_REFUSED instead of embedded Scriptotar UI. Check production Tauri build mode; do not classify the browser error page as app UI.'
      }
      $report['browser_network_error'] = 'ERR_CONNECTION_REFUSED'
    } else {
      $cases.installed_webview = @{
        status='NOT TESTED'
        reason='After 90s, installed shell had no confirmed Scriptotar WebView2 Document + Studio button; inspect child HWNDs, readiness and native screenshot'
      }
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
  if ($cases.installed_webview.status -eq 'PASS') {
    $providerHwnd = [IntPtr]$probe.hwnd
    $tag = 'NativeQA-' + $SourceSha.Substring(0,8)
    $first = $tag + '-A'
    $second = $tag + '-B'
    try {
      Click-InstalledControl $providerHwnd 'Home' ([System.Windows.Automation.ControlType]::Button)
      Wait-InstalledDashboard $providerHwnd 'Inbox'
      foreach ($name in @($first, $second)) {
        Click-InstalledControl $providerHwnd 'New project' ([System.Windows.Automation.ControlType]::Button)
        Type-InstalledEdit $providerHwnd 'Project name' $name
        Click-InstalledControl $providerHwnd 'Create project' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledDashboard $providerHwnd $name
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir "created-$name.png")
      }
      Choose-InstalledProject $providerHwnd $first
      Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'switched-to-A.png')
      Choose-InstalledProject $providerHwnd $second
      Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'switched-to-B.png')
      $cases.create_switch_projects = @{
        status='PASS'
        reason='Two actual installed GUI project creates and A/B switches through project UIA, with visible dashboard checks'
      }
    } catch {
      $cases.create_switch_projects = @{
        status='FAIL'
        reason="Installed WebView2 project interaction: $($_.Exception.Message)"
      }
      Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'failed-create_switch_projects.png')
      Write-UiaDiagnostics $root (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
    }
    if ($cases.create_switch_projects.status -eq 'PASS') {
      try {
        $projectProof = Wait-InstalledCanonicalDb 'projects' $first $second 'unused'
        $cases.canonical_project_persistence = @{
          status='PASS'
          reason=$projectProof
        }
      } catch {
        $cases.canonical_project_persistence = @{
          status='FAIL'
          reason="Installed Windows canonical project persistence: $($_.Exception.Message)"
        }
      }
    }
    if ($cases.create_switch_projects.status -eq 'PASS' -and
        $cases.canonical_project_persistence.status -eq 'PASS') {
      try {
        Choose-InstalledProject $providerHwnd $first
        Click-InstalledControl $providerHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $providerHwnd 'Study name' ([System.Windows.Automation.ControlType]::Edit) | Out-Null
        $studyTitle = $tag + '-Study'
        Type-InstalledEdit $providerHwnd 'Study name' $studyTitle
        Click-InstalledControl $providerHwnd 'Create study' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $providerHwnd 'Duplicate current script' ([System.Windows.Automation.ControlType]::Button) 12 | Out-Null
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'created-study.png')
        $cases.create_study = @{
          status='PASS'
          reason='Study created through installed WebView2 UIA input, button and candidate controls'
        }
      } catch {
        $cases.create_study = @{
          status='FAIL'
          reason="Installed WebView2 Study interaction: $($_.Exception.Message)"
        }
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'failed-create_study.png')
        Write-UiaDiagnostics $root (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
      }
    }

    if ($cases.create_study.status -eq 'PASS') {
      try {
        $draftBody = 'Original installed WebView2 user-edit candidate ' + $tag
        Type-InstalledEdit $providerHwnd 'Script draft' $draftBody
        $saveButton = Find-ExactInstalledControl $providerHwnd 'Save changes' ([System.Windows.Automation.ControlType]::Button)
        # The editor also auto-saves after 900ms; avoid failing when native UI
        # has already saved and disabled the manual button. Database proof is
        # mandatory in either case.
        if ($saveButton.Current.IsEnabled) {
          Click-InstalledControl $providerHwnd 'Save changes' ([System.Windows.Automation.ControlType]::Button)
        }
        $savedProof = Wait-InstalledCanonicalDb 'draft' $first $studyTitle $draftBody
        Click-InstalledControl $providerHwnd 'Duplicate current script' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $providerHwnd 'Rename script' ([System.Windows.Automation.ControlType]::Button) | Out-Null
        Click-InstalledControl $providerHwnd 'Rename script' ([System.Windows.Automation.ControlType]::Button)
        $copyTitle = $tag + '-copy'
        Type-InstalledEdit $providerHwnd 'Script candidate name' $copyTitle
        Click-InstalledControl $providerHwnd 'Save name' ([System.Windows.Automation.ControlType]::Button)
        # Force two real native candidate switches, not a reselect/no-op.
        Choose-InstalledOption $providerHwnd 'Script candidates' $studyTitle
        if ((Read-InstalledEdit $providerHwnd 'Script draft') -ne $draftBody) {
          throw 'Switching to original native candidate changed saved draft body'
        }
        Choose-InstalledOption $providerHwnd 'Script candidates' $copyTitle
        if ((Read-InstalledEdit $providerHwnd 'Script draft') -ne $draftBody) {
          throw 'Renamed native candidate did not preserve duplicated draft body'
        }
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'draft-duplicated-renamed.png')
        $cases.edit_save_duplicate_candidate = @{
          status='PASS'
          reason="Real UIA edit/save, canonical SQLite ($savedProof), duplicate/rename and two candidate-selector switches"
        }
      } catch {
        $cases.edit_save_duplicate_candidate = @{
          status='FAIL'
          reason="Installed WebView2 draft/candidate interaction: $($_.Exception.Message)"
        }
        Save-UnverifiedWindow $root (Join-Path $EvidenceDir 'failed-edit_save_duplicate_candidate.png')
        Write-UiaDiagnostics $root (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
      }
    }

    if ($cases.edit_save_duplicate_candidate.status -eq 'PASS') {
      try {
        # Native Alt+F4 targets only the installed app main HWND; a forced
        # Stop-Process would not prove graphical shutdown or flush behavior.
        if (-not [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle)) {
          throw 'Cannot focus installed WebView2 window for native Alt+F4'
        }
        Start-Sleep -Milliseconds 250
        [System.Windows.Forms.SendKeys]::SendWait('%{F4}')
        if (-not $proc.WaitForExit(12000)) {
          throw 'Installed Windows app did not exit from graphical Alt+F4'
        }
        $proc = Start-Process -FilePath $app.FullName -PassThru
        $probeRestored = $null
        $deadline = [DateTime]::UtcNow.AddSeconds(45)
        do {
          $proc.Refresh()
          if ($proc.HasExited) { throw 'Installed app exited during native relaunch' }
          if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
            [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
            $probeRestored = Find-InstalledDocumentAndStudio $proc
            if ($probeRestored.document -and $probeRestored.studio) { break }
          }
          Start-Sleep -Milliseconds 550
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not $probeRestored -or -not $probeRestored.document -or -not $probeRestored.studio) {
          throw 'Relaunched installed WebView2 has no native Document and Studio control'
        }
        $restoredHwnd = [IntPtr]$probeRestored.hwnd
        Click-InstalledControl $restoredHwnd 'Home' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledDashboard $restoredHwnd $first
        Click-InstalledControl $restoredHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Choose-InstalledOption $restoredHwnd 'Open study' $studyTitle
        Find-ExactInstalledControl $restoredHwnd 'Duplicate current script' ([System.Windows.Automation.ControlType]::Button) 12 | Out-Null
        if ((Read-InstalledEdit $restoredHwnd 'Script draft') -ne $draftBody) {
          throw 'Original Study draft was not restored after true installed GUI restart'
        }
        Choose-InstalledOption $restoredHwnd 'Script candidates' $copyTitle
        if ((Read-InstalledEdit $restoredHwnd 'Script draft') -ne $draftBody) {
          throw 'Renamed draft body was not restored from canonical Study after restart'
        }
        $restoreProof = Wait-InstalledCanonicalDb 'draft' $first $studyTitle $draftBody
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'reopened-candidate.png')
        $cases.quit_relaunch_persistence = @{
          status='PASS'
          reason="Native installed WebView2 Alt+F4, process relaunch, saved project, original/copy candidate and DB restoration ($restoreProof)"
        }
      } catch {
        $cases.quit_relaunch_persistence = @{
          status='FAIL'
          reason="Windows graphical quit/relaunch: $($_.Exception.Message)"
        }
        if ($proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) {
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-quit_relaunch_persistence.png')
          Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
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
