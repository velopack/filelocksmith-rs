fn main() {
    let mut build = cc::Build::new();
    build.cpp(true);
    build.static_crt(true);
    build.include(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\FileLocksmith.cpp");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\NtdllBase.cpp");
    build.file(r"PowerToys\src\modules\FileLocksmith\FileLocksmithLibInterop\NtdllExtensions.cpp");
    build.file(r"src/interop.cpp");
    build.flag_if_supported("/std:c++17");
    build.compile("powertoys_locksmith");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/interop.cpp");
}
