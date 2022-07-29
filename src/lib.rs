mod ioctl;
pub mod watchdog_device;

// Bringing elements into scope
pub use crate::watchdog_device::{Watchdog, OptionFlags, SetOptionFlags};
