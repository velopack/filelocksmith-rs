#include <iostream>
#include <string>
#include "Windows.h"
#include "versionhelpers.h"

extern "C" bool SetDebugPrivilege();
extern "C" char* FindLockingProcessesAtPathAsJson(char* path_utf8, size_t path_length);

int main()
{
	if (!SetDebugPrivilege()) {
		std::cout << "Failed to set Debug Privilege\n";
	}

	char path[] = "C:\\Users\\Caelan\\AppData\\Local\\AvaloniaCrossPlat\\current";
	char* json = FindLockingProcessesAtPathAsJson(path, sizeof(path));
	std::cout << json << std::endl;


	auto asd = HIBYTE(_WIN32_WINNT_WIN10);
	std::cout << std::to_string(asd) << std::endl;

	free(json);
}

