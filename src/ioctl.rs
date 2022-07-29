//! ioctl syatem calls.
//!
//! This module regroups the facilities to access the driver via Linux syscalls.
//! All the ioctl request codes have been obtained from the Linux Kernel headers: 
//!  - include/uapi/linux/watchdog.h
//! Kernel documentation: /Documentation/userspace-api/ioctl/ioctl-number.rst
use libc::c_int;
#[cfg(unix)]
use nix::{ioctl_read, ioctl_readwrite};


const WATCHDOG_IOCTL_BASE: u8 = b'W';

const WDIOC_GETSUPPORT: u8 = 0;
const WDIOC_GETSTATUS: u8 = 1;
const WDIOC_GETBOOTSTATUS: u8 = 2;
const WDIOC_GETTEMP: u8 = 3;
const WDIOC_SETOPTIONS: u8 = 4;
const WDIOC_KEEPALIVE: u8 = 5;
const WDIOC_SETTIMEOUT: u8 = 6;
const WDIOC_GETTIMEOUT: u8 = 7;
const WDIOC_SETPRETIMEOUT: u8 = 8;
const WDIOC_GETPRETIMEOUT: u8 = 9;
const WDIOC_GETTIMELEFT: u8 = 10;

const IDENTITY_STR_LEN: usize = 32;

/// The following struct corresponds to the one defined in the Linux Kernel headers: 
///  - include/uapi/linux/watchdog.h :
/// ```text
/// struct watchdog_info {
/// 	__u32 options;		/* Options the card/driver supports */
/// 	__u32 firmware_version;	/* Firmware version of the card */
/// 	__u8  identity[32];	/* Identity of the board */
/// };
/// ```
#[repr(C)] // see https://docs.rust-embedded.org/book/c-tips/index.html#packed-and-aligned-types
pub struct watchdog_info{
    /// Flags describing what the device supports
    pub options: u32,           
    /// The firmware version of the card if available
    pub firmware_version: u32,  
    /// a string identifying the watchdog driver
    pub identity: [u8; IDENTITY_STR_LEN],      
}

impl watchdog_info {
    pub fn new() -> Self{
        watchdog_info{
            options: 0,
            firmware_version: 0,
            identity: [0; IDENTITY_STR_LEN]
        }
    }
}

ioctl_read!(ioctl_get_support, WATCHDOG_IOCTL_BASE, WDIOC_GETSUPPORT, watchdog_info);
ioctl_read!(ioctl_get_status, WATCHDOG_IOCTL_BASE, WDIOC_GETSTATUS, c_int);
ioctl_read!(ioctl_get_bootstatus, WATCHDOG_IOCTL_BASE, WDIOC_GETBOOTSTATUS, c_int);
ioctl_read!(ioctl_get_temp, WATCHDOG_IOCTL_BASE, WDIOC_GETTEMP, c_int);
ioctl_read!(ioctl_set_options, WATCHDOG_IOCTL_BASE, WDIOC_SETOPTIONS, c_int);
ioctl_read!(ioctl_keepalive, WATCHDOG_IOCTL_BASE, WDIOC_KEEPALIVE, c_int);
ioctl_readwrite!(ioctl_set_timeout, WATCHDOG_IOCTL_BASE, WDIOC_SETTIMEOUT, c_int);
ioctl_read!(ioctl_get_timeout, WATCHDOG_IOCTL_BASE, WDIOC_GETTIMEOUT, c_int);
ioctl_readwrite!(ioctl_set_pretimeout, WATCHDOG_IOCTL_BASE, WDIOC_SETPRETIMEOUT, c_int);
ioctl_read!(ioctl_get_pretimeout, WATCHDOG_IOCTL_BASE, WDIOC_GETPRETIMEOUT, c_int);
ioctl_read!(ioctl_get_time_left, WATCHDOG_IOCTL_BASE, WDIOC_GETTIMELEFT, c_int);
