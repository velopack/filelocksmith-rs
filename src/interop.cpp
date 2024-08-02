#include "pch.h"
#include "Shlobj.h"
#include "FileLocksmith.h"

using namespace std;

static string wstring_to_utf8(wstring const &wstr)
{
    if (wstr.empty())
        return string();
    int size_needed = WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), NULL, 0, NULL, NULL);
    string strTo(size_needed, 0);
    WideCharToMultiByte(CP_UTF8, 0, &wstr[0], (int)wstr.size(), &strTo[0], size_needed, NULL, NULL);
    return strTo;
}

static wstring utf8_to_wstring(string const &str)
{
    if (str.empty())
        return wstring();
    int size_needed = MultiByteToWideChar(CP_UTF8, 0, &str[0], (int)str.size(), NULL, 0);
    wstring strTo(size_needed, 0);
    MultiByteToWideChar(CP_UTF8, 0, &str[0], (int)str.size(), &strTo[0], size_needed);
    return strTo;
}

static char *to_pointer(const std::string &str)
{
    size_t length = str.length() + 1;
    char *copy = (char *)malloc(length);
    if (copy)
    {
        strcpy_s(copy, length, str.c_str());
    }
    return copy;
}

static char *to_pointer(const std::wstring &str)
{
    std::string utf8 = wstring_to_utf8(str);
    return to_pointer(utf8);
}

/* Adapted from "https://learn.microsoft.com/windows/win32/secauthz/enabling-and-disabling-privileges-in-c--" */
extern "C" bool SetDebugPrivilege()
{
    HANDLE hToken;
    TOKEN_PRIVILEGES tp{};
    LUID luid;

    if (OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &hToken) != 0)
    {
        if (!LookupPrivilegeValue(
                NULL,          // lookup privilege on local system
                SE_DEBUG_NAME, // privilege to lookup
                &luid))        // receives LUID of privilege
        {
            CloseHandle(hToken);
            return false;
        }
        tp.PrivilegeCount = 1;
        tp.Privileges[0].Luid = luid;
        tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;

        if (!AdjustTokenPrivileges(
                hToken,
                FALSE,
                &tp,
                sizeof(TOKEN_PRIVILEGES),
                (PTOKEN_PRIVILEGES)NULL,
                (PDWORD)NULL))
        {
            CloseHandle(hToken);
            return false;
        }

        if (GetLastError() == ERROR_NOT_ALL_ASSIGNED)
        {
            CloseHandle(hToken);
            return false;
        }

        CloseHandle(hToken);
        return true;
    }
    return false;
}

extern "C" bool IsProcessElevated()
{
    HANDLE token = nullptr;
    bool elevated = false;
    if (OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token))
    {
        TOKEN_ELEVATION elevation{};
        DWORD size;
        if (GetTokenInformation(token, TokenElevation, &elevation, sizeof(elevation), &size))
        {
            elevated = (elevation.TokenIsElevated != 0);
        }
    }
    if (token)
    {
        CloseHandle(token);
    }
    return elevated;
}

extern "C" void FindProcessesLockingPath(const char *path_utf8, size_t **pids, size_t *count)
{
    string utf8path(path_utf8);
    wstring path = utf8_to_wstring(utf8path);
    vector<wstring> paths{path};
    auto results = find_processes_recursive(paths);
    *count = results.size();

    if (*count == 0)
    {
        *pids = nullptr;
        return;
    }

    *pids = (size_t *)malloc(sizeof(size_t) * (*count));
    for (size_t i = 0; i < *count; i++)
    {
        (*pids)[i] = results[i].pid;
    }
}

extern "C" bool QuitProcesses(const size_t *pids, size_t count)
{
    bool ret = true;
    for (size_t i = 0; i < count; i++)
    {
        size_t pid = pids[i];
        HANDLE hProcess = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
        if (hProcess == NULL)
        {
            ret = false;
        }
        TerminateProcess(hProcess, 0);
        CloseHandle(hProcess);
    }
    return ret;
}

extern "C" char *PidToFullPath(size_t pid)
{
    wstring path = pid_to_full_path(pid);
    if (path.empty())
    {
        return nullptr;
    }
    return to_pointer(path);
}

extern "C" void FreeString(char *ptr)
{
    free(ptr);
}

extern "C" void FreeArray(size_t *ptr)
{
    free(ptr);
}