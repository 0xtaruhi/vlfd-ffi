# C++ Hotplug Demo

This example shows how to consume the `vlfd-ffi` dynamic library from C++.

## Build

```bash
./build.sh
```

The script builds the Rust `cdylib` (`libvlfd_ffi.so`) in release mode and then
compiles `main.cpp`, linking against the library and embedding an rpath so the
binary can locate the shared object at runtime.

## Run

```bash
LD_LIBRARY_PATH=../../target/release ./build/hotplug_demo
```

On systems where `libusb` hotplug support is available, you should see arrival
and removal messages as devices are plugged/unplugged. If the target hardware
is not connected, `vlfd_io_open` will fail gracefully and print the underlying
error from the Rust library.
