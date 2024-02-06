#include <iostream>

extern "C" bool SetDebugPrivilege();
extern "C" bool TryCloseProcessesUsingPath(wchar_t* pszAppName, wchar_t* pszPath);

int main()
{
	if (!SetDebugPrivilege()) {
		std::cout << "Failed to set Debug Privilege\n";
	}

	wchar_t path[] = L"C:\\Users\\Caelan\\AppData\\Local\\AvaloniaCrossPlat\\current";
	wchar_t appName[] = L"AvaloniaCrossPlat";
	TryCloseProcessesUsingPath(appName, path);
}

