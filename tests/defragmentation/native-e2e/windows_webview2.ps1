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
  visual_capture = @{}
  visual_capture_errors = @{}
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
            $info = @{
              hwnd = $hwnd.ToInt64()
              host_class = $className
              depth = $depth
              control_type = $element.Current.ControlType.ProgrammaticName
              name = [string]$element.Current.Name
            }
            # Diagnostic-only UIA evidence: these properties explain why a
            # List rejects SetFocus while a pointer or real keyboard route
            # might still be supported. They NEVER count as an interaction.
            try {
              $info.keyboard_focusable = [bool]$element.Current.IsKeyboardFocusable
              $info.has_keyboard_focus = [bool]$element.Current.HasKeyboardFocus
            } catch { $info.focusability_error = $_.Exception.Message }
            try {
              $bounds = $element.Current.BoundingRectangle
              $info.bounds = @{
                x = $bounds.X; y = $bounds.Y
                width = $bounds.Width; height = $bounds.Height
              }
            } catch { $info.bounds_error = $_.Exception.Message }
            try {
              $info.supported_patterns = @($element.GetSupportedPatterns() | ForEach-Object {
                $_.ProgrammaticName
              })
            } catch { $info.patterns_error = $_.Exception.Message }
            $probe.child_webview_provider_nodes += $info
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

function Find-InstalledDocumentAndStudio([object]$process, [string]$studioName='Studio') {
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
      [System.Windows.Automation.AutomationElement]::NameProperty, $studioName)
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
function Wait-InstalledStudioPageText([IntPtr]$hwnd, [string]$expected) {
  # Chrome WebView2 maps the content h1 as visible Text; the compact nav
  # exposes its rail control as a Button. Require actual main-document text,
  # not just a translated rail control or a language-picker label.
  $matchesName = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::NameProperty, $expected),
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Text))
  $deadline = [DateTime]::UtcNow.AddSeconds(12)
  do {
    try {
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      if ($root) {
        $rootRect = $root.Current.BoundingRectangle
        $textNodes = $root.FindAll([System.Windows.Automation.TreeScope]::Subtree, $matchesName)
        foreach ($element in $textNodes) {
          $rect = $element.Current.BoundingRectangle
          if ($rect.Width -gt 20 -and $rect.Height -gt 10 -and
              $rect.X -gt ($rootRect.X + 80) -and
              -not $element.Current.IsOffscreen) {
            return
          }
        }
      }
    } catch {
      # Provider disconnected during a route/locale re-render; retry.
    }
    Start-Sleep -Milliseconds 350
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed WebView2 Studio main content text missing: '$expected'"
}
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
  # WebView2 often exposes stale/hidden duplicates after a real relaunch.
  # Find a currently visible, enabled UIA control on THIS installed render
  # HWND instead of blindly clicking the first matching AutomationElement.
  $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
  if (-not $root) { throw "Installed UIA render HWND unavailable when clicking '$name'" }
  $condition = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::NameProperty, $name),
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $kind))
  $deadline = [DateTime]::UtcNow.AddSeconds(12)
  $element = $null
  $scrollAttempted = $false
  do {
    try {
      $owner = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      $ownerRect = $owner.Current.BoundingRectangle
      $matches = $owner.FindAll([System.Windows.Automation.TreeScope]::Subtree, $condition)
      $scrollCandidate = $null
      foreach ($candidate in $matches) {
        $bounds = $candidate.Current.BoundingRectangle
        $cx = $bounds.X + $bounds.Width / 2
        $cy = $bounds.Y + $bounds.Height / 2
        if (-not $candidate.Current.IsOffscreen -and $candidate.Current.IsEnabled -and
            $bounds.Width -gt 3 -and $bounds.Height -gt 3 -and
            $ownerRect.Width -gt 3 -and $ownerRect.Height -gt 3 -and
            $cx -ge $ownerRect.X -and $cx -le ($ownerRect.X + $ownerRect.Width) -and
            $cy -ge $ownerRect.Y -and $cy -le ($ownerRect.Y + $ownerRect.Height)) {
          $element = $candidate
          break
        }
        # Run #44150090373: the genuine, restored Script candidates ComboBox
        # was at y=971, below the 720px installed WebView viewport. UIA exposes
        # ScrollItemPattern; do not call this a missing product control.
        if (-not $scrollAttempted -and -not $scrollCandidate -and
            $candidate.Current.IsEnabled -and $bounds.Width -gt 3 -and
            $bounds.Height -gt 3) {
          $scrollCandidate = $candidate
        }
      }
      if (-not $element -and -not $scrollAttempted -and $scrollCandidate) {
        $scrollAttempted = $true
        try {
          $scrollItem = $scrollCandidate.GetCurrentPattern(
            [System.Windows.Automation.ScrollItemPattern]::Pattern)
          if ($scrollItem) {
            $scrollItem.ScrollIntoView()
            Write-Host "Native UIA ScrollIntoView requested for offscreen '$name'"
            Start-Sleep -Milliseconds 400
          }
        } catch {
          Write-Host "Native UIA ScrollIntoView unavailable for '$name': $($_.Exception.Message)"
        }
        # Always requery HWND and freshly rendered accessibility bounds;
        # scrolling by itself is never a verified click or a PASS.
      }
    } catch {
      # Provider may be re-rendering after native relaunch; retry in bounds.
    }
    if ($element) { break }
    Start-Sleep -Milliseconds 300
  } while ([DateTime]::UtcNow -lt $deadline)
  if (-not $element) {
    throw "No visible, enabled, bounded UIA control '$name' ($kind) in installed WebView2 HWND"
  }

  if (-not $pointer) {
    try {
      $invoke = $element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      if ($invoke) {
        $invoke.Invoke()
        Start-Sleep -Milliseconds 550
        return
      }
    } catch {
      # Only a supported InvokePattern succeeds. Unsupported UIA providers
      # require a genuine native pointer action against the verified bounds.
    }
  }

  $bounds = $element.Current.BoundingRectangle
  $cx = $bounds.X + ($bounds.Width / 2)
  $cy = $bounds.Y + ($bounds.Height / 2)
  # GetClickablePoint throws for some real WebView2 accessible elements.
  # Its absence is NOT a skipped click: fall back to the verified visible
  # rectangle center, then let the following native UI assertions decide.
  try {
    $clickable = $element.GetClickablePoint()
    if (-not [Double]::IsNaN($clickable.X) -and
        -not [Double]::IsNaN($clickable.Y) -and
        $clickable.X -ge $bounds.X -and
        $clickable.X -le ($bounds.X + $bounds.Width) -and
        $clickable.Y -ge $bounds.Y -and
        $clickable.Y -le ($bounds.Y + $bounds.Height)) {
      $cx = $clickable.X
      $cy = $clickable.Y
    }
  } catch {
    Write-Host "UIA GetClickablePoint unsupported for '$name'; using visible, enabled HWND-bounded UIA rectangle"
  }
  [NativeWebViewWindow]::NativeClick([int][Math]::Round($cx), [int][Math]::Round($cy))
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
function Paste-InstalledQueueUrl([IntPtr]$hwnd, [string]$url) {
  # Installed WebView2 sometimes keeps the URL placeholder after SetFocus +
  # SendKeys.SendWait (run #44308361975, attached screenshot + UIA Edit).
  # Demand an actual HWND-bounded pointer click and native clipboard paste.
  # Never use ValuePattern.SetValue, JavaScript injection, or direct backend API.
  Click-InstalledControl $hwnd 'Media URL' ([System.Windows.Automation.ControlType]::Edit) -pointer
  $field = Find-ExactInstalledControl $hwnd 'Media URL' ([System.Windows.Automation.ControlType]::Edit)
  $field.SetFocus()
  Set-Clipboard -Value $url
  [System.Windows.Forms.SendKeys]::SendWait('^a')
  [System.Windows.Forms.SendKeys]::SendWait('^v')
  Start-Sleep -Milliseconds 750
  $actual = [string]$field.GetCurrentPattern(
    [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
  if ($actual -cne $url) {
    throw "Installed Media URL did not receive actual native clipboard input (expected length $($url.Length), actual length $($actual.Length), focused=$($field.Current.HasKeyboardFocus))"
  }
}
function Paste-InstalledJson([IntPtr]$hwnd, [string]$name, [string]$value) {
  # Never invoke UIA SetValue: it bypasses browser input handlers. SendKeys
  # cannot type literal JSON braces safely, so use real native clipboard paste
  # into the focused WebView2 editor, then independently read its ValuePattern.
  $field = Find-ExactInstalledControl $hwnd $name ([System.Windows.Automation.ControlType]::Edit)
  $field.SetFocus()
  Set-Clipboard -Value $value
  [System.Windows.Forms.SendKeys]::SendWait('^a')
  [System.Windows.Forms.SendKeys]::SendWait('^v')
  Start-Sleep -Milliseconds 650
  $actual = [string]$field.GetCurrentPattern(
    [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
  if ($actual -cne $value) {
    throw "Installed JSON editor '$name' did not receive exact native pasted text"
  }
}
function Wait-InstalledAngleRejection([IntPtr]$hwnd) {
  # Match only a visible rendered error associated with angle validation.
  # A malformed fixture alone is not proof of a denied IPC.
  $textCondition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
    [System.Windows.Automation.ControlType]::Text)
  $deadline = [DateTime]::UtcNow.AddSeconds(12)
  do {
    try {
      $tree = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      if ($tree) {
        $nodes = $tree.FindAll([System.Windows.Automation.TreeScope]::Subtree, $textCondition)
        foreach ($item in $nodes) {
          $label = [string]$item.Current.Name
          if ($label -match '(?i)(angle.*invalid|invalid.*angle)' -and
              -not $item.Current.IsOffscreen) { return }
        }
      }
    } catch { }
    Start-Sleep -Milliseconds 350
  } while ([DateTime]::UtcNow -lt $deadline)
  throw 'Malformed three-angle JSON produced no visible installed UIA validation error'
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
  # The installed popup exposes an actual UIA List / ListItem; its List does
  # NOT support SetFocus on Windows WebView2 (run #44118342637). Use the
  # accessible option's verified screen rectangle for a real pointer click.
  # UIA SetValue or a synthetic DOM change would not exercise Svelte input.
  $ancestor = $option
  $list = $null
  for ($depth=0; $depth -lt 12; $depth++) {
    $ancestor = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($ancestor)
    if (-not $ancestor) { break }
    if ($ancestor.Current.ControlType -eq [System.Windows.Automation.ControlType]::List) {
      $list = $ancestor
      break
    }
  }
  if (-not $list) {
    throw "Native WebView2 option '$name' is not owned by an accessible popup List"
  }
  $rect = $option.Current.BoundingRectangle
  $listRect = $list.Current.BoundingRectangle
  if ($rect.Width -le 2 -or $rect.Height -le 2 -or
      $listRect.Width -le 2 -or $listRect.Height -le 2) {
    throw "Native WebView2 option '$name' or owner List has no actionable screen rectangle"
  }
  $cx = $rect.X + ($rect.Width / 2)
  $cy = $rect.Y + ($rect.Height / 2)
  if ($cx -lt $listRect.X -or $cx -gt ($listRect.X + $listRect.Width) -or
      $cy -lt $listRect.Y -or $cy -gt ($listRect.Y + $listRect.Height)) {
    throw "Native WebView2 option '$name' lies outside its accessible popup List"
  }
  [NativeWebViewWindow]::NativeClick([int][Math]::Round($cx), [int][Math]::Round($cy))
  Wait-InstalledPopupClosed $hwnd $name

}
function Test-InstalledPopupOptionVisible([IntPtr]$hwnd, [string]$name) {
  # Inspect the exact option within its actual List ancestor. The installed
  # project ComboBox 'PROJECT' opens a List named 'Project'; never assume
  # that a popup List and its corresponding ComboBox share an accessible name.
  $provider = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
  if (-not $provider) { throw "Installed WebView2 provider unavailable while verifying popup" }
  $match = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::NameProperty, $name)
  $matches = $provider.FindAll([System.Windows.Automation.TreeScope]::Subtree, $match)
  foreach ($element in $matches) {
    $kind = $element.Current.ControlType
    if ($kind -notin @(
        [System.Windows.Automation.ControlType]::ListItem,
        [System.Windows.Automation.ControlType]::Button)) { continue }
    $rect = $element.Current.BoundingRectangle
    if ($rect.Width -le 2 -or $rect.Height -le 2) { continue }
    $ancestor = $element
    for ($depth=0; $depth -lt 12; $depth++) {
      $ancestor = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($ancestor)
      if (-not $ancestor) { break }
      if ($ancestor.Current.ControlType -eq [System.Windows.Automation.ControlType]::List) {
        return $true
      }
    }
  }
  return $false
}
function Wait-InstalledPopupClosed([IntPtr]$hwnd, [string]$name) {
  # No provider errors count as closed. A still-visible option under its real
  # List fails even if a coincidentally correct dashboard is already visible.
  $deadline = [DateTime]::UtcNow.AddSeconds(10)
  do {
    try {
      if (-not (Test-InstalledPopupOptionVisible $hwnd $name)) { return }
    } catch {
      # A disconnected provider is not evidence of successful user selection.
    }
    Start-Sleep -Milliseconds 300
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed native popup option '$name' remained visible or UIA provider disconnected after pointer selection"
}
function Choose-InstalledProject([IntPtr]$hwnd, [string]$name) {
  Choose-InstalledOption $hwnd 'PROJECT' $name
  Wait-InstalledDashboard $hwnd $name
}
function Wait-InstalledComboValue([IntPtr]$hwnd, [string]$label, [string]$expected) {
  # Merely reading the script body cannot prove native candidate switching:
  # a cloned candidate can have the identical text. Inspect the rendered
  # selected label INSIDE the actual installed combobox's screen bounds.
  # A popup option is outside that rectangle and does not count as selected.
  $textMatch = [System.Windows.Automation.AndCondition]::new(
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::NameProperty, $expected),
    [System.Windows.Automation.PropertyCondition]::new(
      [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
      [System.Windows.Automation.ControlType]::Text))
  $deadline = [DateTime]::UtcNow.AddSeconds(10)
  do {
    try {
      $combo = Find-ExactInstalledControl $hwnd $label ([System.Windows.Automation.ControlType]::ComboBox) 2
      $bounds = $combo.Current.BoundingRectangle
      $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
      $matches = $root.FindAll([System.Windows.Automation.TreeScope]::Subtree, $textMatch)
      $selected = $false
      foreach ($text in $matches) {
        $rect = $text.Current.BoundingRectangle
        $cx = $rect.X + ($rect.Width / 2)
        $cy = $rect.Y + ($rect.Height / 2)
        if ($rect.Width -gt 2 -and $rect.Height -gt 2 -and
            $cx -ge $bounds.X -and $cx -le ($bounds.X + $bounds.Width) -and
            $cy -ge $bounds.Y -and $cy -le ($bounds.Y + $bounds.Height)) {
          $selected = $true
          break
        }
      }
      # A still-open popup must not count as a completed selection.
      # A mismatched List accessible name must not let an open popup
      # masquerade as a completed candidate selection.
      $stillOpen = Test-InstalledPopupOptionVisible $hwnd $expected
      if ($selected -and -not $stillOpen) { return }
    } catch { }
    Start-Sleep -Milliseconds 350
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed native combobox '$label' did not select and close on '$expected'"
}
function Read-InstalledEdit([IntPtr]$hwnd, [string]$name) {
  $field = Find-ExactInstalledControl $hwnd $name ([System.Windows.Automation.ControlType]::Edit)
  return [string]$field.GetCurrentPattern(
    [System.Windows.Automation.ValuePattern]::Pattern).Current.Value
}
function Wait-InstalledEditExact([IntPtr]$hwnd, [string]$name, [string]$expected, [int]$seconds=12) {
  # The installed WebView2 Studio can expose an existing Study and Scripts 2
  # while the editor's asynchronous value is still settling after A->B->A.
  # Require exact native UIA ValuePattern text, never a screenshot or DB-only
  # substitute. Keep a bounded wait and fail closed with content length only.
  $deadline = [DateTime]::UtcNow.AddSeconds($seconds)
  $actual = ''
  $lastError = ''
  do {
    try {
      $actual = Read-InstalledEdit $hwnd $name
      if ($actual -ceq $expected) { return }
    } catch {
      $lastError = $_.Exception.Message
    }
    Start-Sleep -Milliseconds 350
  } while ([DateTime]::UtcNow -lt $deadline)
  throw "Installed UIA edit '$name' did not restore exact expected content (expected length $($expected.Length), actual length $($actual.Length), last UIA error: $lastError)"
}
function Verify-InstalledCanonicalDb([string]$kind, [string]$first, [string]$second, [string]$third, [string]$fourth='') {
  # Windows CI has Python 3.12, and this imports only standard library code.
  # The canonical probe opens installed app SQLite in mode=ro and never
  # changes user state, shortcuts project selection or fabricates GUI PASS.
  $qaRoot = (Resolve-Path 'tests/defragmentation/native-e2e').Path
  $probe = @'
import sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
from linux_webkit import (
    verify_canonical_projects,
    verify_canonical_saved_draft,
    verify_canonical_reopened_candidates,
    count_canonical_jobs,
)
kind, root, a, b, c, copy = sys.argv[2:8]
if kind == "projects":
    print(verify_canonical_projects(Path(root), (a, b)))
elif kind == "draft":
    print(verify_canonical_saved_draft(Path(root), a, b, c))
elif kind == "candidates":
    print(verify_canonical_reopened_candidates(Path(root), a, b, c, copy))
elif kind == "jobs":
    print(count_canonical_jobs(Path(root), a))
else:
    raise ValueError("Unsupported canonical installed DB probe")
'@
  $result = & python -c $probe $qaRoot $kind $dataDir $first $second $third $fourth 2>&1
  if ($LASTEXITCODE -ne 0) {
    throw "Installed canonical SQLite $kind probe failed: $($result | Out-String)"
  }
  return [string]($result | Out-String).Trim()
}
function Wait-InstalledCanonicalDb([string]$kind, [string]$first, [string]$second, [string]$third, [string]$fourth='') {
  # The real UI can complete an auto-save shortly after a native key event.
  # Poll the authoritative SQLite snapshot instead of racing an inaccessible
  # status label or interpreting a temporarily disabled save button as failure.
  $lastError = 'Not checked'
  for ($attempt=0; $attempt -lt 24; $attempt++) {
    try {
      return Verify-InstalledCanonicalDb $kind $first $second $third $fourth
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
        Wait-InstalledComboValue $providerHwnd 'Script candidates' $studyTitle
        if ((Read-InstalledEdit $providerHwnd 'Script draft') -ne $draftBody) {
          throw 'Switching to original native candidate changed saved draft body'
        }
        Choose-InstalledOption $providerHwnd 'Script candidates' $copyTitle
        Wait-InstalledComboValue $providerHwnd 'Script candidates' $copyTitle
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
        # Exact installed UIA tree at run #44159845627 retained Studio,
        # project A, Study, two scripts and the Script candidates ComboBox,
        # but candidate bounds were y=971 below the 749px document viewport.
        # ScrollItemPattern alone did not bring that control onscreen.
        # Use the genuine, visible Studio 'Scripts 2' hyperlink, then still
        # require the candidate control, exact body and immutable SQLite.
        Click-InstalledControl $restoredHwnd 'Scripts 2' ([System.Windows.Automation.ControlType]::Hyperlink)
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'reopened-scripts-navigation.png')
        Find-ExactInstalledControl $restoredHwnd 'Duplicate current script' ([System.Windows.Automation.ControlType]::Button) 12 | Out-Null
        if ((Read-InstalledEdit $restoredHwnd 'Script draft') -ne $draftBody) {
          throw 'Original Study draft was not restored after true installed GUI restart'
        }
        Choose-InstalledOption $restoredHwnd 'Script candidates' $copyTitle
        Wait-InstalledComboValue $restoredHwnd 'Script candidates' $copyTitle
        if ((Read-InstalledEdit $restoredHwnd 'Script draft') -ne $draftBody) {
          throw 'Renamed draft body was not restored from canonical Study after restart'
        }
        $restoreProof = Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle
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
    # The following two scenarios are downstream of a fully validated
    # production Study, saved draft and genuine graphical restart. A failure
    # above cannot be compensated by success here.
    if ($cases.quit_relaunch_persistence.status -eq 'PASS') {
      try {
        # Real project switch and UIA screenshot, not a forged IPC call.
        Click-InstalledControl $restoredHwnd 'Home' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledDashboard $restoredHwnd $first
        Choose-InstalledProject $restoredHwnd $second
        Click-InstalledControl $restoredHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $restoredHwnd 'Study name' ([System.Windows.Automation.ControlType]::Edit) 12 | Out-Null
        $match = [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, $studyTitle)
        $rootB = [System.Windows.Automation.AutomationElement]::FromHandle($restoredHwnd)
        $foreign = $rootB.FindAll([System.Windows.Automation.TreeScope]::Subtree, $match)
        foreach ($element in $foreign) {
          $rect = $element.Current.BoundingRectangle
          if ($rect.Width -gt 2 -and $rect.Height -gt 2 -and
              -not $element.Current.IsOffscreen) {
            throw "Foreign Study '$studyTitle' visible within project B's native Studio"
          }
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'wrong-project-ui-isolation.png')
        # Return to A through real GUI and independently verify its saved data
        # still exists. Seeing an empty page alone is not isolation proof.
        Click-InstalledControl $restoredHwnd 'Home' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledDashboard $restoredHwnd $second
        Choose-InstalledProject $restoredHwnd $first
        Click-InstalledControl $restoredHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Choose-InstalledOption $restoredHwnd 'Open study' $studyTitle
        # Integration #44176696340 showed the exact Study title and Scripts 2
        # visibly restored, but an immediate single UIA read was not the
        # expected body. Navigate the real editor link, then poll the actual
        # installed ValuePattern for bounded asynchronous hydration.
        Click-InstalledControl $restoredHwnd 'Scripts 2' ([System.Windows.Automation.ControlType]::Hyperlink)
        Wait-InstalledEditExact $restoredHwnd 'Script draft' $draftBody
        $isolationProof = Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle
        $cases.wrong_project_ui_isolation = @{
          status='PASS'
          reason="Installed UIA project A->B, no visible foreign Study in B, returned to A with original Study and $isolationProof"
        }
      } catch {
        $cases.wrong_project_ui_isolation = @{
          status='FAIL'
          reason="Windows installed wrong-project UI isolation: $($_.Exception.Message)"
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-wrong_project_ui_isolation.png')
        Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
      }
    }
    if ($cases.wrong_project_ui_isolation.status -eq 'PASS') {
      try {
        # Actual artifact #14378911964 shows the real candidate ComboBox at
        # y=80, partially hidden behind the fixed Studio toolbar (bottom=111),
        # after the wrong-project isolation navigation. Clicking its UIA
        # rectangle center would hit the toolbar, not the candidate menu.
        # Scroll the genuine installed WebView2 Document upward via its native
        # UI Automation ScrollPattern, then require the entire ComboBox below
        # the fixed toolbar before any pointer action. No injected DOM/JS.
        $candidateReady = $false
        for ($attempt=0; $attempt -lt 5; $attempt++) {
          $frame = [System.Windows.Automation.AutomationElement]::FromHandle($restoredHwnd)
          if (-not $frame) { throw 'Installed native WebView2 HWND missing while locating candidate' }
          $document = $frame.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            [System.Windows.Automation.PropertyCondition]::new(
              [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
              [System.Windows.Automation.ControlType]::Document))
          if (-not $document) { throw 'Installed WebView2 document unavailable during candidate navigation' }
          $candidate = $frame.FindFirst(
            [System.Windows.Automation.TreeScope]::Subtree,
            [System.Windows.Automation.AndCondition]::new(
              [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::NameProperty, 'Script candidates'),
              [System.Windows.Automation.PropertyCondition]::new(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::ComboBox)))
          if (-not $candidate) { throw 'Installed Script candidates ComboBox absent after project restoration' }
          $rect = $candidate.Current.BoundingRectangle
          $viewport = $frame.Current.BoundingRectangle
          if ($candidate.Current.IsEnabled -and $rect.Width -gt 3 -and
              $rect.Height -gt 3 -and $rect.Y -ge ($viewport.Y + 90) -and
              $rect.Bottom -le ($viewport.Bottom - 5)) {
            $candidateReady = $true
            break
          }
          $scroll = $document.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
          if (-not $scroll) { throw 'Installed document has no native UIA ScrollPattern to reveal Script candidates' }
          $scroll.Scroll(
            [System.Windows.Automation.ScrollAmount]::NoAmount,
            [System.Windows.Automation.ScrollAmount]::SmallDecrement)
          Start-Sleep -Milliseconds 350
        }
        if (-not $candidateReady) {
          throw 'Native Script candidates ComboBox remained obscured by fixed toolbar after bounded UIA scroll'
        }
        # Actually open the Svelte custom listbox and send a native Escape
        # keystroke to the installed foreground WebView2 window.
        Choose-InstalledOption $restoredHwnd 'Script candidates' $copyTitle
        Wait-InstalledComboValue $restoredHwnd 'Script candidates' $copyTitle
        Click-InstalledControl $restoredHwnd 'Script candidates' ([System.Windows.Automation.ControlType]::ComboBox) -pointer
        if (-not (Test-InstalledPopupOptionVisible $restoredHwnd $studyTitle)) {
          throw 'Installed WebView2 candidate list did not open with original option'
        }
        [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
        Wait-InstalledPopupClosed $restoredHwnd $studyTitle
        Wait-InstalledComboValue $restoredHwnd 'Script candidates' $copyTitle
        if ((Read-InstalledEdit $restoredHwnd 'Script draft') -ne $draftBody) {
          throw 'Escape changed the actual installed selected candidate draft body'
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'keyboard-custom-controls.png')
        $cases.keyboard_custom_controls = @{
          status='PASS'
          reason='Installed WebView2 custom option opened, native Escape closed popup, selected candidate and body remained unchanged'
        }
      } catch {
        $cases.keyboard_custom_controls = @{
          status='FAIL'
          reason="Windows installed custom keyboard control: $($_.Exception.Message)"
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-keyboard_custom_controls.png')
        Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
      }
    }
    if ($cases.keyboard_custom_controls.status -eq 'PASS') {
      try {
        Click-InstalledControl $restoredHwnd 'Settings' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $restoredHwnd 'Theme' ([System.Windows.Automation.ControlType]::ComboBox) 12 | Out-Null
        # SettingsView in this exact app source offers System and Dark only.
        # A Light label is not invented or accepted as evidence.
        foreach ($choice in @('System', 'Dark')) {
          Choose-InstalledOption $restoredHwnd 'Theme' $choice
          Wait-InstalledComboValue $restoredHwnd 'Theme' $choice
          $saveTheme = Find-ExactInstalledControl $restoredHwnd 'Save changes' ([System.Windows.Automation.ControlType]::Button)
          if ($saveTheme.Current.IsEnabled) {
            Click-InstalledControl $restoredHwnd 'Save changes' ([System.Windows.Automation.ControlType]::Button)
          }
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir ("windows-theme-" + $choice.ToLowerInvariant() + ".png"))
        }
        # Capture the genuine installed open theme menu (not a web mock).
        # A screenshot is review material, NEVER automatic visual PASS.
        try {
          Click-InstalledControl $restoredHwnd 'Theme' ([System.Windows.Automation.ControlType]::ComboBox) -pointer
          if (-not (Test-InstalledPopupOptionVisible $restoredHwnd 'System')) {
            throw 'Expected actual System option in open native theme menu'
          }
          $themeMenuShot = Join-Path $EvidenceDir 'windows-theme-menu-open.png'
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) $themeMenuShot
          if (-not (Test-Path $themeMenuShot)) { throw 'Installed theme menu screenshot not captured' }
          $report.visual_capture['dark_theme_menu'] = 'captured'
        } catch {
          $report.visual_capture_errors['dark_theme_menu'] = $_.Exception.Message
        } finally {
          [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
          Start-Sleep -Milliseconds 200
        }
        # Certify saved theme state across a SECOND genuine installed process
        # shutdown/relaunch. A visual selector click in one process is not enough.
        if (-not [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle)) {
          throw 'Cannot activate installed WebView2 window for theme persistence restart'
        }
        Start-Sleep -Milliseconds 250
        [System.Windows.Forms.SendKeys]::SendWait('%{F4}')
        if (-not $proc.WaitForExit(12000)) {
          throw 'Installed Windows app did not close with Alt+F4 during theme persistence check'
        }
        $proc = Start-Process -FilePath $app.FullName -PassThru
        $themeProbe = $null
        $themeDeadline = [DateTime]::UtcNow.AddSeconds(45)
        do {
          $proc.Refresh()
          if ($proc.HasExited) { throw 'Installed app exited during theme persistence relaunch' }
          if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
            [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
            $themeProbe = Find-InstalledDocumentAndStudio $proc
            if ($themeProbe.document -and $themeProbe.studio) { break }
          }
          Start-Sleep -Milliseconds 550
        } while ([DateTime]::UtcNow -lt $themeDeadline)
        if (-not $themeProbe -or -not $themeProbe.document -or -not $themeProbe.studio) {
          throw 'Relaunched WebView2 failed native document/Studio gate for theme persistence'
        }
        $themeHwnd = [IntPtr]$themeProbe.hwnd
        Click-InstalledControl $themeHwnd 'Settings' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledComboValue $themeHwnd 'Theme' 'Dark'
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-theme-dark-reopened.png')
        $cases.theme_mode_control = @{
          status='PASS'
          reason='Real installed WebView2 Settings System/Dark selection and Save, Alt+F4/relaunch, Dark still selected; screenshots require separate visual review'
        }
      } catch {
        $cases.theme_mode_control = @{
          status='FAIL'
          reason="Windows native Settings theme and persistence: $($_.Exception.Message)"
        }
        if ($proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) {
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-theme_mode_control.png')
          Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
        }
      }
    }
    if ($cases.theme_mode_control.status -eq 'PASS') {
      try {
        # Use the installed shell's real language combobox. Switching only a
        # topbar EN/ع marker without the Studio content heading never passes.
        Choose-InstalledOption $themeHwnd 'Interface language' 'ع'
        Wait-InstalledComboValue $themeHwnd 'لغة الواجهة' 'ع'
        Click-InstalledControl $themeHwnd 'الاستوديو' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledStudioPageText $themeHwnd 'الاستوديو'
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-arabic-locale.png')
        # Preserve actual native RTL popup placement for independent review.
        # Not a PASS assertion: dialog, focus, and offscreen views still matter.
        try {
          Click-InstalledControl $themeHwnd 'لغة الواجهة' ([System.Windows.Automation.ControlType]::ComboBox) -pointer
          if (-not (Test-InstalledPopupOptionVisible $themeHwnd 'EN')) {
            throw 'Expected actual EN option in open Arabic locale menu'
          }
          $arabicMenuShot = Join-Path $EvidenceDir 'windows-arabic-menu-open.png'
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) $arabicMenuShot
          if (-not (Test-Path $arabicMenuShot)) { throw 'Installed Arabic menu screenshot not captured' }
          $report.visual_capture['arabic_menu'] = 'captured'
        } catch {
          $report.visual_capture_errors['arabic_menu'] = $_.Exception.Message
        } finally {
          [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
          Start-Sleep -Milliseconds 200
        }

        # Genuinely quit and relaunch with saved Arabic locale. The native
        # document/rail gate must now seek Arabic rather than English.
        if (-not [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle)) {
          throw 'Cannot activate installed WebView2 window for Arabic persistence restart'
        }
        Start-Sleep -Milliseconds 250
        [System.Windows.Forms.SendKeys]::SendWait('%{F4}')
        if (-not $proc.WaitForExit(12000)) {
          throw 'Installed app did not quit through Alt+F4 while Arabic'
        }
        $proc = Start-Process -FilePath $app.FullName -PassThru
        $arabicProbe = $null
        $arabicDeadline = [DateTime]::UtcNow.AddSeconds(45)
        do {
          $proc.Refresh()
          if ($proc.HasExited) { throw 'Installed app exited during Arabic persisted relaunch' }
          if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
            [NativeWebViewWindow]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
            $arabicProbe = Find-InstalledDocumentAndStudio $proc 'الاستوديو'
            if ($arabicProbe.document -and $arabicProbe.studio) { break }
          }
          Start-Sleep -Milliseconds 550
        } while ([DateTime]::UtcNow -lt $arabicDeadline)
        if (-not $arabicProbe -or -not $arabicProbe.document -or -not $arabicProbe.studio) {
          throw 'Installed WebView2 did not restore Arabic Studio rail and native Document'
        }
        $arabicHwnd = [IntPtr]$arabicProbe.hwnd
        Click-InstalledControl $arabicHwnd 'الاستوديو' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledStudioPageText $arabicHwnd 'الاستوديو'
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-arabic-reopened.png')

        Choose-InstalledOption $arabicHwnd 'لغة الواجهة' 'EN'
        Wait-InstalledComboValue $arabicHwnd 'Interface language' 'EN'
        Click-InstalledControl $arabicHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledStudioPageText $arabicHwnd 'Studio'
        # The main content must no longer expose a visible Arabic title.
        $arabicHeading = [System.Windows.Automation.PropertyCondition]::new(
          [System.Windows.Automation.AutomationElement]::NameProperty, 'الاستوديو')
        $uiRoot = [System.Windows.Automation.AutomationElement]::FromHandle($arabicHwnd)
        foreach ($element in $uiRoot.FindAll([System.Windows.Automation.TreeScope]::Subtree, $arabicHeading)) {
          $rect = $element.Current.BoundingRectangle
          if ($element.Current.ControlType -eq [System.Windows.Automation.ControlType]::Text -and
              $rect.Width -gt 20 -and $rect.Height -gt 10 -and
              -not $element.Current.IsOffscreen) {
            throw 'Arabic Studio text still visibly mounted after English restoration'
          }
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-english-restored.png')
        $cases.arabic_locale_control = @{
          status='PASS'
          reason='Installed WebView2 Arabic native Studio, graphical Arabic relaunch, English shell and Studio main content restoration; screenshots do not alone certify full RTL'
        }
      } catch {
        $cases.arabic_locale_control = @{
          status='FAIL'
          reason="Windows native Arabic/English installed locale journey: $($_.Exception.Message)"
        }
        if ($proc -and $proc.MainWindowHandle -ne [IntPtr]::Zero) {
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-arabic_locale_control.png')
          Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
        }
      }
    }
    if ($cases.arabic_locale_control.status -eq 'PASS') {
      try {
        # Deterministic no-cost negative/recovery through actual installed
        # WebView2 inputs and the public Rust Study angle review command.
        Click-InstalledControl $arabicHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Choose-InstalledOption $arabicHwnd 'Open study' $studyTitle
        Type-InstalledEdit $arabicHwnd 'Creative goal' 'Describe practical original editing tips'
        Type-InstalledEdit $arabicHwnd 'Audience' 'People learning video editing'
        Click-InstalledControl $arabicHwnd 'Prepare 3 creative angles' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $arabicHwnd 'Paste three-angle JSON response' ([System.Windows.Automation.ControlType]::Edit) | Out-Null
        Paste-InstalledJson $arabicHwnd 'Paste three-angle JSON response' '{"angles":[{"id":"only-one-angle","approach":"Invalid","hook":"Incomplete","promise":"Missing two angles","outline":[{"role":"hook","intent":"Should be rejected","estimated_seconds":7}],"evidence_ids":[]}]}'
        Click-InstalledControl $arabicHwnd 'Validate 3 angles' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledAngleRejection $arabicHwnd
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-negative-angles-rejected.png')
        Paste-InstalledJson $arabicHwnd 'Paste three-angle JSON response' '{"angles":[{"id":"angle-recovery-problem","approach":"Explain an everyday problem","hook":"Start with the overlooked problem","promise":"Make the context clear","outline":[{"role":"hook","intent":"Introduce the everyday problem","estimated_seconds":7}],"evidence_ids":[]},{"id":"angle-recovery-contrast","approach":"Contrast two useful approaches","hook":"Compare two possible choices","promise":"Show a practical distinction","outline":[{"role":"hook","intent":"Open with an original contrast","estimated_seconds":7}],"evidence_ids":[]},{"id":"angle-recovery-checklist","approach":"Share a practical checklist","hook":"Three steps to get started","promise":"Give practical next actions","outline":[{"role":"hook","intent":"Introduce a practical checklist","estimated_seconds":7}],"evidence_ids":[]}]}'
        Click-InstalledControl $arabicHwnd 'Validate 3 angles' ([System.Windows.Automation.ControlType]::Button)
        Find-ExactInstalledControl $arabicHwnd 'Share a practical checklist' ([System.Windows.Automation.ControlType]::Text) 12 | Out-Null
        Wait-InstalledEditExact $arabicHwnd 'Script draft' $draftBody
        $proof = Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'windows-three-angles-recovered.png')
        # Use real installed UIA against the production URL admission path.
        # file: is deliberately unsupported. This must NOT create a job or
        # mutate the previously persisted Study/candidates.
        $beforeJobs = [int](Verify-InstalledCanonicalDb 'jobs' $first '' '')
        Click-InstalledControl $arabicHwnd 'Queue' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledStudioPageText $arabicHwnd 'Jobs'
        Paste-InstalledQueueUrl $arabicHwnd 'file:///scriptotar-native-qa-denied-media.mp4'
        Click-InstalledControl $arabicHwnd 'Queue URL' ([System.Windows.Automation.ControlType]::Button)
        $deniedError = ''
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
          $owner = [System.Windows.Automation.AutomationElement]::FromHandle($arabicHwnd)
          $tree = $owner.FindAll(
            [System.Windows.Automation.TreeScope]::Subtree,
            [System.Windows.Automation.Condition]::TrueCondition)
          foreach ($element in $tree) {
            try {
              if ($element.Current.ControlType -ne [System.Windows.Automation.ControlType]::Text -or
                  $element.Current.IsOffscreen) { continue }
              $value = [string]$element.Current.Name
              if ($value.Length -gt 12 -and
                  $value -match '(?i)(scheme|protocol|unsupported|invalid|blocked|not allowed|denied|file:)' -and
                  $value -ne 'Supported media URL') {
                $deniedError = $value
                break
              }
            } catch {
              continue
            }
          }
          if ($deniedError) { break }
          Start-Sleep -Milliseconds 350
        } while ([DateTime]::UtcNow -lt $deadline)
        if (-not $deniedError) {
          Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-denied-url-no-error.png')
          throw 'Unsupported URL submitted through installed Queue but no visible UIA error text appeared'
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'rejected-url-error.png')
        $afterJobs = [int](Verify-InstalledCanonicalDb 'jobs' $first '' '')
        if ($afterJobs -ne $beforeJobs) {
          throw "Denied URL mutated canonical job count (before=$beforeJobs, after=$afterJobs)"
        }
        Click-InstalledControl $arabicHwnd 'Studio' ([System.Windows.Automation.ControlType]::Button)
        Wait-InstalledStudioPageText $arabicHwnd 'Studio'
        Choose-InstalledOption $arabicHwnd 'Open study' $studyTitle
        Wait-InstalledEditExact $arabicHwnd 'Script draft' $draftBody
        $recovered = Verify-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'recovered-studio-after-denial.png')
        $report.denied_url_error = $deniedError.Substring(0, [Math]::Min(300, $deniedError.Length))
        $cases.error_recovery = @{
          status='PASS'
          reason="Installed WebView2 three-angle and Queue negative paths passed; production Queue rejected unsupported file URL with visible UIA error, zero new canonical jobs and original Study/candidate recovered ($recovered)"
        }
      } catch {
        $cases.error_recovery = @{
          status='FAIL'
          reason="Installed WebView2 three-angle and URL recovery: $($_.Exception.Message)"
        }
        Save-UnverifiedWindow ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'failed-error_recovery.png')
        Write-UiaDiagnostics ([System.Windows.Automation.AutomationElement]::FromHandle($proc.MainWindowHandle)) (Join-Path $EvidenceDir 'uia-diagnostics.json') $proc
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
