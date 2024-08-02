#pragma once

#include <string>

struct CProcessInfo;

bool SetDebugPrivilege();
bool IsProcessElevated();
// bool PidToFullPath(uint32_t pid);