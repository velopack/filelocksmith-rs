#include "pch.h"
#include "Shlobj.h"

#include "FileLocksmith.h"
#include "file_locksmith/src/lib.rs.h"

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
bool SetDebugPrivilege()
{
	HANDLE hToken;
	TOKEN_PRIVILEGES tp{};
	LUID luid;

	if (OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &hToken) != 0)
	{
		if (!LookupPrivilegeValue(
				NULL,		   // lookup privilege on local system
				SE_DEBUG_NAME, // privilege to lookup
				&luid))		   // receives LUID of privilege
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

// adapted from common/utils/elevation.h. No need to bring all dependencies to this project, though.
// TODO: Make elevation.h lighter so that this function can be used without bringing dependencies like spdlog in.
bool IsProcessElevated()
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

bool CloseProcesses(vector<ProcessResult> &processes)
{
	for (auto &process : processes)
	{
		HANDLE hProcess = OpenProcess(PROCESS_TERMINATE, FALSE, process.pid);
		if (hProcess == NULL)
		{
			return false;
		}
		TerminateProcess(hProcess, 0);
		CloseHandle(hProcess);
	}
	return true;
}

std::string PidToFullPath(size_t pid)
{
	wstring path = pid_to_full_path(pid);
	return wstring_to_utf8(path);
}