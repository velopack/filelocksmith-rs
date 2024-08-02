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

    // let target = std::env::var("TARGET").unwrap();
    // match target.as_str() {
    //     "i686-pc-windows-msvc" => {
    //         build.define("_WIN32", None);
    //     }
    //     "x86_64-pc-windows-msvc" => {
    //         build.define("_WIN64", None);
    //     }
    //     "aarch64-pc-windows-msvc" => {
    //         build.define("_ARM64", None);
    //     }
    //     _ => panic!("Unsupported target architecture"),
    // }

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    match target_arch.as_str() {
        "x86" => build.define("_WIN32", None),
        "x86_64" => build.define("_WIN64", None),
        "aarch64" => build.define("_ARM64", None),
        _ => panic!("Unsupported target architecture: {}", target_arch),
    };

    build.compile("powertoys_locksmith");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/interop.cpp");
}
