fn main() {
    let target = std::env::var("TARGET").unwrap();
    let mut build = cxx_build::bridge("src/lib.rs");

    build.cpp(true);
    build.include(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop");
    build.include(r"include");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\FileLocksmith.cpp");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\NtdllBase.cpp");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\NtdllExtensions.cpp");
    build.file(r"src/interop.cpp");
    build.flag_if_supported("/std:c++17");

    match target.as_str() {
        "i686-pc-windows-msvc" => {
            build.define("_WIN32", None);
        }
        "x86_64-pc-windows-msvc" => {
            build.define("_WIN64", None);
        }
        "aarch64-pc-windows-msvc" => {
            build.define("_ARM64", None);
        }
        _ => panic!("Unsupported target architecture"),
    }

    build.compile("powertoys_locksmith");
}
