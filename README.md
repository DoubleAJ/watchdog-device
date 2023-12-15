# watchdog-device
![Build](https://github.com/DoubleAJ/watchdog-device/actions/workflows/build.yml/badge.svg) 
[![Crate](https://img.shields.io/crates/v/watchdog-device.svg)](https://crates.io/crates/watchdog-device)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![API](https://docs.rs/watchdog-device/badge.svg)](https://docs.rs/watchdog-device)

Linux Watchdog API Rust implementation.

This library facilitates the usage of the Watchdog driver API provided by the Linux Kernel.
The watchdog is used to automatically verify whether a program is running as expected. 
The following text was readapted from the [`Linux Kernel Documentation`]:

A Watchdog Timer (WDT) is a hardware circuit that can reset the computer system in case of a software fault.
Usually a userspace daemon will notify the kernel watchdog driver that userspace is still alive, at regular intervals. 
When such a notification occurs, the driver will usually tell the hardware watchdog that everything is in order, 
and that the watchdog should wait for yet another little while to reset the system. 
If userspace fails (RAM error, kernel bug, whatever), the notifications cease to occur, 
and the hardware watchdog will reset the system (causing a reboot) after the timeout occurs.

In case of the absence of a hardware watchdog, the Linux Kernel offers a software implementation via the `softdog` module.
It can be loaded by calling:
```text
# modprobe softdog
```

## Usage
To integrate this library to your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
watchdog-device = "0.2.0"
```

A watchdog is available if any `/dev/watchdog*` file is present in the system. In order to use it, the program must be executed as a user who has read/write permissions on it.

It is possible to have more that one Watchdog. In addition to `/dev/watchdog`, there could be other files named with a numerical suffix (e.g.: `/dev/watchdog0` , `/dev/watchdog1`, etc.).
The function `Watchdog::new()` allows the activation of the default watchdog (represented by the file with no suffix).
The function `Watchdog::new_by_id()` allows the activation of a specific watchdog (represented by a file with a suffix) by indicating the numerical ID as parameter.

All drivers support the basic mode of operation, where the watchdog activates as soon as a `Watchdog` instance is created 
and will reboot unless the watchdog is pinged within a certain time, this time is called the timeout or margin. 
The simplest way to ping the watchdog is to call the `keep_alive()` method.

When the device is closed, the watchdog is disabled, unless the “Magic Close” feature is supported (see below). 
This is not always such a good idea, since if there is a bug in the watchdog daemon and it crashes the system will not reboot. 
Because of this, some of the drivers support the configuration option “Disable watchdog shutdown on close”, CONFIG_WATCHDOG_NOWAYOUT. 
If it is set to Y when compiling the kernel, there is no way of disabling the watchdog once it has been started. 
So, if the watchdog daemon crashes, the system will reboot after the timeout has passed. 
Watchdog devices also usually support the nowayout module parameter so that this option can be controlled at runtime.

## Magic Close feature
If a driver supports 'Magic Close', the driver will not disable the watchdog 
unless `magic_close()` is called just before releasing the watchdog instance. 
If the userspace daemon closes the watchdog without calling `magic_close()`, 
the driver will assume that the daemon (and userspace in general) died, and will stop pinging the watchdog without disabling it first. 
This will then cause a reboot if the watchdog is not re-opened in sufficient time.

## Example

```rust
use watchdog_device::Watchdog;
use nix::errno::Errno;


let mut wd = Watchdog::new()?;
loop{
    do_something();
    if let Err(e) = wd.keep_alive(){
        println!("Error {}", e);
    }
}
```

[`Linux Kernel Documentation`]: https://www.kernel.org/doc/html/latest/watchdog/watchdog-api.html

## Testing
A series of integration tests are available. 

By default `cargo test` runs tests in parallel. Since they would interfere with each other, it is important to run them one at a time:
```bash
$ cargo test -- --test-threads=1
```

## License

This project is [licensed under the MIT license](https://github.com/DoubleAJ/watchdog-device/blob/main/LICENSE).
