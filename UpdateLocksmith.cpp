#include "pch.h"

#include "PowerToys/src/modules/FileLocksmith/FileLocksmithLibInterop/FileLocksmith.h"

#include "Commctrl.h"
#pragma comment(lib, "comctl32.lib")

/* Adapted from "https://learn.microsoft.com/windows/win32/secauthz/enabling-and-disabling-privileges-in-c--" */
extern "C" bool SetDebugPrivilege()
{
	HANDLE hToken;
	TOKEN_PRIVILEGES tp{};
	LUID luid;

	if (OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &hToken) != 0)
	{
		if (!LookupPrivilegeValue(
			NULL, // lookup privilege on local system
			SE_DEBUG_NAME, // privilege to lookup
			&luid)) // receives LUID of privilege
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

bool CloseProcesses(std::vector<ProcessResult>& processes)
{
	for (auto& process : processes)
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

extern "C" bool TryCloseProcessesUsingPath(wchar_t* pszAppName, wchar_t* pszPath)
{
	std::wstring path(pszPath);
	std::wstring appName(pszAppName);
	std::vector<std::wstring> paths{ path };

	while (true) {
		auto results = find_processes_recursive(paths);
		auto numResults = results.size();

		if (numResults == 0)
		{
			return true;
		}

		int nButtonPressed = 0;
		TASKDIALOGCONFIG config = { 0 };
		const TASKDIALOG_BUTTON buttons[] = {
			{ IDRETRY, L"Retry\nTry again if you've closed the program(s)"},
			{ IDOK, L"Continue\nAttempt to close the program(s) automatically" },
			{ IDCANCEL, L"Cancel\nThe update will not continue" },
		};

		std::wstring message = L"There is a program (" + results[0].name + L" [" + std::to_wstring(results[0].pid)
			+ L"]) preventing the " + appName + L" update from proceeding."
			+ L"\n\nYou can press Continue to attempt closing it automatically, or close it yourself and then press Retry.";

		if (numResults > 1)
		{
			message = L"There are " + std::to_wstring(numResults) + L" programs preventing the " + appName + L" update from proceeding:\n";
			for (size_t i = 0; i < numResults; i++)
			{
				message += L"\n" + results[i].name + L" [" + std::to_wstring(results[i].pid) + L"]";
			}
			message += L"\n\nYou can press Continue to attempt closing them automatically, or close them yourself and then press Retry.";
		}

		std::wstring title = appName + L" Update";
		std::wstring instruction = appName + L" Update";

		config.cbSize = sizeof(config);
		config.hInstance = GetModuleHandle(NULL);
		config.pszMainIcon = TD_INFORMATION_ICON;
		config.pszMainInstruction = instruction.c_str();
		config.pszWindowTitle = title.c_str();
		config.pszContent = message.c_str();
		config.dwFlags = TDF_USE_COMMAND_LINKS | TDF_ALLOW_DIALOG_CANCELLATION;
		config.pButtons = buttons;
		config.cButtons = ARRAYSIZE(buttons);

		TaskDialogIndirect(&config, &nButtonPressed, NULL, NULL);
		switch (nButtonPressed)
		{
		case IDOK:
			CloseProcesses(results);
			// retry now
			break;
		case IDRETRY:
			// retry now
			break;
		default: // cancel or anything else
			return false;
		}
	}
}