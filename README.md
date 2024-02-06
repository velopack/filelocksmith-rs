Used by [Velopack](https://github.com/velopack/velopack) to detect running processes which might be locking an update directory.

This internally uses/exposes PowerToys FileLocksmith functionality as a C static lib with a function easily consumed by Rust code.