"""OS process-memory snapshots for installed-app benchmark instrumentation.

Linux: proportional set size (PSS) and private resident memory via
/proc/<pid>/smaps_rollup; Windows: private working set and private commit from
PROCESS_MEMORY_COUNTERS_EX2. Missing/denied child data invalidates tree totals.
Process samples are not evidence of useful native Studio UI readiness.
"""
from __future__ import annotations

import ctypes
from pathlib import Path
import platform


def parse_smaps_rollup(text: str) -> dict[str, int | None]:
    values: dict[str, int] = {}
    for line in text.splitlines():
        key, sep, value = line.partition(":")
        if not sep or key not in ("Pss", "Private_Clean", "Private_Dirty"):
            continue
        parts = value.strip().split()
        if len(parts) == 2 and parts[0].isdigit() and parts[1] == "kB":
            values[key] = int(parts[0]) * 1024
    private = (values["Private_Clean"] + values["Private_Dirty"]
               if "Private_Clean" in values and "Private_Dirty" in values else None)
    return {"pss_bytes": values.get("Pss"), "private_resident_bytes": private}


def descendants(root_pid: int, parents: dict[int, int]) -> set[int]:
    if root_pid not in parents:
        return set()
    chosen = {root_pid}
    while True:
        new = {pid for pid, ppid in parents.items() if ppid in chosen} - chosen
        if not new:
            return chosen
        chosen.update(new)


def aggregate_samples(samples: dict[int, dict[str, int | None]],
                      pids: set[int], field: str) -> int | None:
    """Never disguise a partial process-tree measurement as complete."""
    if not pids or any(pid not in samples or samples[pid].get(field) is None for pid in pids):
        return None
    return sum(int(samples[pid][field]) for pid in pids)


def _linux_processes() -> dict[int, int]:
    parents = {}
    for node in Path("/proc").iterdir():
        if not node.name.isdigit():
            continue
        try:
            status = (node / "status").read_text(encoding="ascii")
            line = next(row for row in status.splitlines() if row.startswith("PPid:"))
            parents[int(node.name)] = int(line.split()[1])
        except (OSError, UnicodeError, StopIteration, ValueError, IndexError):
            continue
    return parents


def linux_memory(root_pid: int) -> dict:
    chosen = descendants(root_pid, _linux_processes())
    if not chosen:
        return {"sampled_process_count": 0, "main_pss_bytes": None,
                "tree_pss_bytes": None, "main_private_resident_bytes": None,
                "tree_private_resident_bytes": None, "tree_memory_complete": False}
    samples = {}
    for pid in chosen:
        try:
            samples[pid] = parse_smaps_rollup(
                Path(f"/proc/{pid}/smaps_rollup").read_text(encoding="ascii"))
        except (OSError, UnicodeError):
            samples[pid] = {"pss_bytes": None, "private_resident_bytes": None}
    main = samples[root_pid]
    tree_pss = aggregate_samples(samples, chosen, "pss_bytes")
    tree_private = aggregate_samples(samples, chosen, "private_resident_bytes")
    return {"sampled_process_count": len(chosen),
            "main_pss_bytes": main["pss_bytes"], "tree_pss_bytes": tree_pss,
            "main_private_resident_bytes": main["private_resident_bytes"],
            "tree_private_resident_bytes": tree_private,
            "tree_memory_complete": tree_pss is not None and tree_private is not None}


def _windows_processes() -> dict[int, int]:
    from ctypes import wintypes as w
    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [("dwSize", w.DWORD), ("cntUsage", w.DWORD),
                    ("th32ProcessID", w.DWORD), ("th32DefaultHeapID", ctypes.c_size_t),
                    ("th32ModuleID", w.DWORD), ("cntThreads", w.DWORD),
                    ("th32ParentProcessID", w.DWORD), ("pcPriClassBase", w.LONG),
                    ("dwFlags", w.DWORD), ("szExeFile", w.WCHAR * 260)]
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    kernel.CreateToolhelp32Snapshot.restype = w.HANDLE
    kernel.Process32FirstW.argtypes = [w.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    kernel.Process32FirstW.restype = w.BOOL
    kernel.Process32NextW.argtypes = [w.HANDLE, ctypes.POINTER(PROCESSENTRY32W)]
    kernel.Process32NextW.restype = w.BOOL
    kernel.CloseHandle.argtypes = [w.HANDLE]
    snapshot = kernel.CreateToolhelp32Snapshot(0x2, 0)  # TH32CS_SNAPPROCESS
    if not snapshot or snapshot == ctypes.c_void_p(-1).value:
        return {}
    parents = {}
    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(entry)
    try:
        ok = kernel.Process32FirstW(snapshot, ctypes.byref(entry))
        while ok:
            parents[int(entry.th32ProcessID)] = int(entry.th32ParentProcessID)
            ok = kernel.Process32NextW(snapshot, ctypes.byref(entry))
    finally:
        kernel.CloseHandle(snapshot)
    return parents


def _windows_memory_for_pid(pid: int) -> dict[str, int | None]:
    from ctypes import wintypes as w
    class COUNTERS(ctypes.Structure):
        _fields_ = [("cb", w.DWORD), ("PageFaultCount", w.DWORD),
                    ("PeakWorkingSetSize", ctypes.c_size_t),
                    ("WorkingSetSize", ctypes.c_size_t),
                    ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                    ("QuotaPagedPoolUsage", ctypes.c_size_t),
                    ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                    ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                    ("PagefileUsage", ctypes.c_size_t),
                    ("PeakPagefileUsage", ctypes.c_size_t),
                    ("PrivateUsage", ctypes.c_size_t),
                    ("PrivateWorkingSetSize", ctypes.c_size_t),
                    ("SharedCommitUsage", ctypes.c_uint64)]
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    kernel.OpenProcess.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    kernel.OpenProcess.restype = w.HANDLE
    kernel.CloseHandle.argtypes = [w.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.c_void_p, w.DWORD]
    psapi.GetProcessMemoryInfo.restype = w.BOOL
    handle = kernel.OpenProcess(0x1010, False, pid)  # QUERY_LIMITED_INFORMATION | VM_READ
    if not handle:
        return {"private_working_set_bytes": None, "private_commit_bytes": None}
    try:
        counters = COUNTERS()
        counters.cb = ctypes.sizeof(counters)
        if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
            return {"private_working_set_bytes": None, "private_commit_bytes": None}
        return {"private_working_set_bytes": int(counters.PrivateWorkingSetSize),
                "private_commit_bytes": int(counters.PrivateUsage)}
    finally:
        kernel.CloseHandle(handle)


def windows_memory(root_pid: int) -> dict:
    chosen = descendants(root_pid, _windows_processes())
    if not chosen:
        return {"sampled_process_count": 0, "main_private_working_set_bytes": None,
                "tree_private_working_set_bytes": None, "main_private_commit_bytes": None,
                "tree_private_commit_bytes": None, "tree_memory_complete": False}
    samples = {pid: _windows_memory_for_pid(pid) for pid in chosen}
    main = samples[root_pid]
    ws = aggregate_samples(samples, chosen, "private_working_set_bytes")
    commit = aggregate_samples(samples, chosen, "private_commit_bytes")
    return {"sampled_process_count": len(chosen),
            "main_private_working_set_bytes": main["private_working_set_bytes"],
            "tree_private_working_set_bytes": ws,
            "main_private_commit_bytes": main["private_commit_bytes"],
            "tree_private_commit_bytes": commit,
            "tree_memory_complete": ws is not None and commit is not None}


def memory_snapshot(root_pid: int) -> dict:
    if platform.system() == "Linux":
        return {"provider": "linux-proc-smaps_rollup", **linux_memory(root_pid)}
    if platform.system() == "Windows":
        return {"provider": "windows-psapi-process_memory_counters_ex2",
                **windows_memory(root_pid)}
    return {"provider": "unsupported", "sampled_process_count": 0,
            "tree_memory_complete": False}
