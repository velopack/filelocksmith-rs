#[cxx::bridge]
mod ffi {
    struct CProcessInfo {
        name: String,
        pid: usize,
    }

    unsafe extern "C++" {
        // One or more headers with the matching C++ declarations. Our code
        // generators don't read it but it gets #include'd and used in static
        // assertions to ensure our picture of the FFI boundary is accurate.
        include!("interop.h");

        // // Zero or more opaque types which both languages can pass around but
        // // only C++ can see the fields.
        // type BlobstoreClient;

        // // Functions implemented in C++.
        // fn new_blobstore_client() -> UniquePtr<BlobstoreClient>;
        // fn put(&self, parts: &mut MultiBuf) -> u64;
        // fn tag(&self, blobid: u64, tag: &str);
        // fn metadata(&self, blobid: u64) -> BlobMetadata;

        fn SetDebugPrivilege() -> bool;
        fn IsProcessElevated() -> bool;
        // fn FindProcessesLockingPath(path: String) -> Vec<CProcessInfo>;
        // fn CloseProcesses(lockedProcesses: Vec<CProcessInfo>) -> bool;
        // fn PidToFullPath(pid: u32) -> bool;
    }
}
