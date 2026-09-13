"""Windows process accounting via Job Objects (ctypes). Import only on Windows.

A sample's process is created suspended, assigned to a fresh job object, then
resumed, so every descendant is accounted. The job gives exact total user/kernel
CPU and I/O transfer counts; peak working set is sampled with psutil by the
runner (Windows has no per-job peak working set counter), and the direct child's
``PeakWorkingSetSize`` is read from its handle after exit as a floor.
"""

from __future__ import annotations

import ctypes
from ctypes import wintypes
from typing import Any, Dict, Optional

kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
ntdll = ctypes.WinDLL("ntdll")
psapi = ctypes.WinDLL("psapi", use_last_error=True)

CREATE_SUSPENDED = 0x00000004
JobObjectBasicAndIoAccountingInformation = 8
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JobObjectExtendedLimitInformation = 9


class IO_COUNTERS(ctypes.Structure):
    _fields_ = [(n, ctypes.c_ulonglong) for n in (
        "ReadOperationCount", "WriteOperationCount", "OtherOperationCount",
        "ReadTransferCount", "WriteTransferCount", "OtherTransferCount")]


class JOBOBJECT_BASIC_ACCOUNTING_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("TotalUserTime", ctypes.c_longlong),
        ("TotalKernelTime", ctypes.c_longlong),
        ("ThisPeriodTotalUserTime", ctypes.c_longlong),
        ("ThisPeriodTotalKernelTime", ctypes.c_longlong),
        ("TotalPageFaultCount", wintypes.DWORD),
        ("TotalProcesses", wintypes.DWORD),
        ("ActiveProcesses", wintypes.DWORD),
        ("TotalTerminatedProcesses", wintypes.DWORD),
    ]


class JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION(ctypes.Structure):
    _fields_ = [("BasicInfo", JOBOBJECT_BASIC_ACCOUNTING_INFORMATION), ("IoInfo", IO_COUNTERS)]


class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_longlong),
        ("PerJobUserTimeLimit", ctypes.c_longlong),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
        ("IoInfo", IO_COUNTERS),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class PROCESS_MEMORY_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
    ]


kernel32.CreateJobObjectW.restype = wintypes.HANDLE
kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
kernel32.QueryInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD, ctypes.c_void_p]
kernel32.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
kernel32.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
ntdll.NtResumeProcess.argtypes = [wintypes.HANDLE]
psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESS_MEMORY_COUNTERS), wintypes.DWORD]


class Job:
    def __init__(self) -> None:
        self.handle = kernel32.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        kernel32.SetInformationJobObject(self.handle, JobObjectExtendedLimitInformation, ctypes.byref(info), ctypes.sizeof(info))

    def assign_and_resume(self, process_handle: int) -> None:
        if not kernel32.AssignProcessToJobObject(self.handle, wintypes.HANDLE(process_handle)):
            err = ctypes.get_last_error()
            ntdll.NtResumeProcess(wintypes.HANDLE(process_handle))
            raise ctypes.WinError(err)
        status = ntdll.NtResumeProcess(wintypes.HANDLE(process_handle))
        if status != 0:
            raise OSError(f"NtResumeProcess failed: NTSTATUS {status:#x}")

    def accounting(self) -> Dict[str, Any]:
        info = JOBOBJECT_BASIC_AND_IO_ACCOUNTING_INFORMATION()
        if not kernel32.QueryInformationJobObject(self.handle, JobObjectBasicAndIoAccountingInformation, ctypes.byref(info), ctypes.sizeof(info), None):
            raise ctypes.WinError(ctypes.get_last_error())
        ext = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        kernel32.QueryInformationJobObject(self.handle, JobObjectExtendedLimitInformation, ctypes.byref(ext), ctypes.sizeof(ext), None)
        b = info.BasicInfo
        return {
            "user_s": b.TotalUserTime / 1e7,
            "sys_s": b.TotalKernelTime / 1e7,
            "page_faults": int(b.TotalPageFaultCount),
            "total_processes": int(b.TotalProcesses),
            "io_read_bytes": int(info.IoInfo.ReadTransferCount),
            "io_write_bytes": int(info.IoInfo.WriteTransferCount),
            "io_read_ops": int(info.IoInfo.ReadOperationCount),
            "io_write_ops": int(info.IoInfo.WriteOperationCount),
            "peak_process_commit_bytes": int(ext.PeakProcessMemoryUsed),
            "peak_job_commit_bytes": int(ext.PeakJobMemoryUsed),
        }

    def terminate(self, code: int = 1) -> None:
        kernel32.TerminateJobObject(self.handle, code)

    def close(self) -> None:
        if self.handle:
            kernel32.CloseHandle(self.handle)
            self.handle = None


def peak_working_set(process_handle: int) -> Optional[int]:
    pmc = PROCESS_MEMORY_COUNTERS()
    pmc.cb = ctypes.sizeof(pmc)
    if psapi.GetProcessMemoryInfo(wintypes.HANDLE(process_handle), ctypes.byref(pmc), pmc.cb):
        return int(pmc.PeakWorkingSetSize)
    return None
