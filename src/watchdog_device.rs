//! Linux Watchdog API implementation.
//!
//! This library facilitates the usage of the Watchdog driver API provided by the Linux Kernel.
//! The watchdog is used to automatically verify whether a program is running as expected. 
//! The following text was readapted from the [`Linux Kernel Documentation`]:
//! 
//! A Watchdog Timer (WDT) is a hardware circuit that can reset the computer system in case of a software fault.
//! Usually a userspace daemon will notify the kernel watchdog driver that userspace is still alive, at regular intervals. 
//! When such a notification occurs, the driver will usually tell the hardware watchdog that everything is in order, 
//! and that the watchdog should wait for yet another little while to reset the system. 
//! If userspace fails (RAM error, kernel bug, whatever), the notifications cease to occur, 
//! and the hardware watchdog will reset the system (causing a reboot) after the timeout occurs.
//! 
//! In case of the absence of a hardware watchdog, the Linux Kernel offers a software implementation via the `softdog` module.
//! It can be loaded by calling:
//! ```text
//! ## modprobe softdog
//! ```
//! 
//! ## Usage
//! All drivers support the basic mode of operation, where the watchdog activates as soon as a [`Watchdog`] instance is created 
//! and will reboot unless the watchdog is pinged within a certain time, this time is called the timeout or margin. 
//! The simplest way to ping the watchdog is to call the [`keep_alive()`](crate::watchdog_device::Watchdog::keep_alive) method.
//! 
//! When the device is closed, the watchdog is disabled, unless the “Magic Close” feature is supported (see below). 
//! This is not always such a good idea, since if there is a bug in the watchdog daemon and it crashes the system will not reboot. 
//! Because of this, some of the drivers support the configuration option “Disable watchdog shutdown on close”, CONFIG_WATCHDOG_NOWAYOUT. 
//! If it is set to Y when compiling the kernel, there is no way of disabling the watchdog once it has been started. 
//! So, if the watchdog daemon crashes, the system will reboot after the timeout has passed. 
//! Watchdog devices also usually support the nowayout module parameter so that this option can be controlled at runtime.
//! 
//! ## Magic Close feature
//! If a driver supports 'Magic Close', the driver will not disable the watchdog 
//! unless [`magic_close()`](crate::watchdog_device::Watchdog::magic_close) is called just before releasing the watchdog instance. 
//! If the userspace daemon closes the watchdog without calling [`magic_close()`](crate::watchdog_device::Watchdog::magic_close), 
//! the driver will assume that the daemon (and userspace in general) died, and will stop pinging the watchdog without disabling it first. 
//! This will then cause a reboot if the watchdog is not re-opened in sufficient time.
//! 
//! # Examples
//! 
//! ```rust
//! use watchdog_device::Watchdog;
//! use nix::errno::Errno;
//! 
//! # fn do_something(){}
//! # fn main() -> Result<(), std::io::Error> {
//! let mut wd = Watchdog::new()?;
//! loop{
//!     do_something();
//!     if let Err(e) = wd.keep_alive(){
//!         println!("Error {}", e);
//!     }
//! #   break;
//! }
//! # wd.magic_close()?;
//! # Ok(())
//! # }
//! ```
//! 
//! [`Linux Kernel Documentation`]: https://www.kernel.org/doc/html/latest/watchdog/watchdog-api.html

use log::{error, warn, info, trace};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::fmt;
use libc::c_int;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::sync::{Arc, Mutex, mpsc::Sender, mpsc::channel, mpsc::RecvTimeoutError};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use nix::errno::Errno;
use crate::ioctl::*;

/// List of all available options that can be supported by a watchdog driver.
/// 
/// From the Linux Kernel Watchdog API documentation:
/// 
/// All watchdog drivers are required return more information about the system, some do temperature, fan and power level monitoring, 
/// some can tell you the reason for the last reboot of the system. 
///
/// All options and their related values have been obtained from the Linux Kernel headers: 
///  - include/uapi/linux/watchdog.h in struct watchdog_info.options
pub enum OptionFlags{
    /// Reset due to CPU overheat
    Overheat,       
    /// Fan failed
    FanFault,       
    /// External relay 1
    Extern1,        
    /// External relay 2
    Extern2,       
    /// Power bad/power fault
    PowerUnder,    
    /// Card previously reset the CPU
    CardReset,      
    /// Power over voltage
    PowerOver,     
    /// Set timeout (in seconds)
    SetTimeout,     
    /// Supports magic close char
    MagicClose,     
    /// Pretimeout (in seconds), get/set
    PreTimeout,     
    /// Watchdog triggers a management or other external alarm not a reboot
    AlarmOnly,      
    /// Keep alive ping reply
    KeepalivePing,  
}

impl OptionFlags{
    fn value(&self) -> u32{
        match self{
            Self::Overheat        => 0x0001,
            Self::FanFault        => 0x0002,   
            Self::Extern1         => 0x0004,   
            Self::Extern2         => 0x0008,   
            Self::PowerUnder      => 0x0010,   
            Self::CardReset       => 0x0020,   
            Self::PowerOver       => 0x0040,   
            Self::SetTimeout      => 0x0080,   
            Self::MagicClose      => 0x0100,
            Self::PreTimeout      => 0x0200,
            Self::AlarmOnly       => 0x0400,
            Self::KeepalivePing   => 0x8000,
        }
    }
}

impl fmt::Display for OptionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Overheat => write!(f, "Overheat"),
            Self::FanFault => write!(f, "FanFault"),
            Self::Extern1 => write!(f, "Extern1"),
            Self::Extern2 => write!(f, "Extern2"),
            Self::PowerUnder => write!(f, "PowerUnder"),
            Self::CardReset => write!(f, "CardReset"),
            Self::PowerOver => write!(f, "PowerOver"),
            Self::SetTimeout => write!(f, "SetTimeout"),
            Self::MagicClose => write!(f, "MagicClose"),
            Self::PreTimeout => write!(f, "PreTimeout"),
            Self::AlarmOnly => write!(f, "AlarmOnly"),
            Self::KeepalivePing => write!(f, "KeepalivePing"),
       }
    }
}

/// The following are all the flags that can be set by using [`Watchdog::set_option()`](crate::watchdog_device::Watchdog::set_option).
pub enum SetOptionFlags{
    /// Turn off the watchdog timer
    DisableCard,     
    /// Turn on the watchdog timer
    EnableCard,     
    /// Kernel panic on temperature trip
    TempPanic,      
}

impl SetOptionFlags{
    fn value(&self) -> u32{
        match self{
            Self::DisableCard   => 0x0001,
            Self::EnableCard    => 0x0002,   
            Self::TempPanic     => 0x0004,   
        }
    }
}

impl fmt::Display for SetOptionFlags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::DisableCard => write!(f, "DisableCard"),
            Self::EnableCard => write!(f, "EnableCard"),
            Self::TempPanic => write!(f, "TempPanic"),
       }
    }
}

enum BitmaskQueryType{
    GetStatus,
    GetBootStatus,
}

enum IntGetterType{
    GetTimeout,
    GetPreTimeout,
    GetTimeLeft,
    GetTemp,
}

/// Structure representing the watchdog.
/// 
/// When opening the file representing the watchdog driver in the Linux filesystem, 
/// the watchdog activates and needs to be pinged to avoid a system reset.
pub struct Watchdog{
    /// File that activates the watchdog when opened.
    file: File,
    /// Message passing utility used to tell the 'automatic keepalive' thread when to exit.
    /// This is used only when calling [`start_automatic_keep_alive()`](Self::start_automatic_keep_alive), hence the 'Option'.
    msg_sender: Option<Sender<()>>,
}

impl Watchdog {
    /// Instantiates the default watchdog.
    /// 
    /// The creation of the instance causes the activation of the watchdog.
    /// Since this involves opening the '/dev/watchdog' file representing the driver, 
    /// the user must have the appropriate read/write permissions to access it. If this is not the case, an error will be returned.
    /// After this call, the only way to prevent a system reset is to periodically call [`keep_alive()`](Self::keep_alive)
    /// before the configured timeout elapses (see [`get_timeout()`](Self::get_timeout) and [`get_time_left()`](Self::get_time_left)).
    /// 
    /// If the 'magic close' feature is supported (see [`is_option_supported()`](Self::is_option_supported) to verify), 
    /// it is possible to deactivate the watchdog by calling [`magic_close()`](Self::magic_close).
    /// 
    /// Once the watchdog is active, an alternative way to keep the system alive is to call 
    /// [`start_automatic_keep_alive()`](Self::start_automatic_keep_alive) just once.
    /// See the documentation of each method for more information.
    pub fn new() -> Result<Self, io::Error>{
        Self::new_instance(None)
    }

    /// Instantiates a specific watchdog with a numeric identifier.
    ///
    /// Unlike [`new()`](Self::new), it creates a watchdog instance by opening the
    /// '/dev/watchdogID' file (e.g. '/dev/watchdog0', '/dev/watchdog37', etc.).
    /// The ID passed as parameter indicates the number suffix of the watchdog file.
    /// As with [`new()`](Self::new), The creation of the instance causes the activation of the watchdog.
    /// See [`new()`](Self::new) for more information.
    pub fn new_by_id(id: u8) -> Result<Self, io::Error>{
        Self::new_instance(Some(id))
    }
    
    fn new_instance(id: Option<u8>) -> Result<Self, io::Error>{
        let mut path = String::from("/dev/watchdog");
        if let Some(id_val) = id {
            path.push_str(&id_val.to_string());
        }
        let f = OpenOptions::new().write(true).open(&path)?;
        warn!("Watchdog:{path} activated.");
        Ok(Self{file: f, msg_sender: Option::None})
    }

    /// Keeps the system alive.
    ///
    /// The watchdog automatically triggers a system reset if not pinged for a preconfigured timeout 
    /// (see [`get_timeout()`](Self::get_timeout) and [`get_time_left()`](Self::get_time_left)).
    /// In order to prevent this, this method must be called periodically before the timeout expires.
    pub fn keep_alive(&mut self) -> Result<(), Errno>{
        let result;
        // The following could also be achieved with: self.file.write(b"0");
        unsafe{
            result = ioctl_keepalive(self.file.as_raw_fd(), std::ptr::null_mut::<c_int>());
        }
        match result{
            Ok(_) => {
                trace!("Keep alive.");
                Ok(())
            },
            Err(e) => Err(e),
        }
    }

    /// Starts automatically keeping the system alive.
    /// 
    /// In a normal operation, the user should periodically call [`keep_alive()`](Self::keep_alive) to prevent the watchdog from triggering a system reset.
    /// When calling this, a separate thread is spawned that takes care of pinging the watchdog once every second.
    /// 
    /// The 'auto keep alive' thread is signaled to be closed as soon as the watchdog instance is released from memory. 
    /// This means that without triggering the [`magic_close()`](Self::magic_close) feature, releasing the watchdog will still cause a system reset after the timeout period.
    ///
    /// **Disclaimer**: this feature should only be considered if the user is sure that their use case will not defeat the purpose of having a watchdog in the first place.
    /// As an example, if the main thread malfunctions but the 'auto keep alive' thread is able to keep running, 
    /// the watchdog will still be pinged normally and no reset will take place. 
    /// This is clearly an undersirable behaviour, so particular caution must be taken when using this.
    ///
    /// # Panics
    /// This method can panic in case the passed mutex is poisoned. 
    /// The same also can happen inside the spawned thread.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use std::sync::{Arc, Mutex};
    /// 
    /// # fn do_something(){}
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// let wd_mutex_arc: Arc<Mutex<Watchdog>> = Arc::new(Mutex::new(wd));
    /// let handle = Watchdog::start_automatic_keep_alive(wd_mutex_arc.clone());
    /// loop{
    ///     do_something();
    /// #   break;
    /// }
    /// # wd_mutex_arc.lock().expect("Error obtaining lock guard.").magic_close()?;
    /// handle.join().expect("Error joining thread.");
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_automatic_keep_alive(watchdog_mut_arc: Arc<Mutex<Self>>) -> JoinHandle<()>{
        let (tx, rx) = channel::<()>();
        watchdog_mut_arc.lock().expect("Couldn't lock the watchdog mutex to set the sender.").msg_sender = Some(tx);
        let handle = thread::spawn(move || {
            info!("Automatic keepalive thread started.");
            let mut keepalive_error_counter = 0;
            loop{
                if let Err(e) = watchdog_mut_arc.lock().expect("Couldn't lock the watchdog mutex to keep alive.").keep_alive(){
                    warn!("Keep alive error {}.", e);
                    keepalive_error_counter += 1;
                    if keepalive_error_counter >= 10{
                        error!("Max number of consecutive keepalive errors reached. Closing thread...");
                        break;
                    }
                }
                else{
                    keepalive_error_counter = 0;
                }
                // These two 'errors' are used as information, so it is not needed to send actual messages.
                if let Err(e) =  rx.recv_timeout(Duration::from_secs(1)){
                    if e == RecvTimeoutError::Timeout{ 
                        trace!("timeout 1s...");
                    }
                    else{
                        // The sender being dropped is an implicit signal that this thread must close.
                        warn!("Sender was terminated. Closing 'auto keepalive' thread...");
                        break;
                    }
                } // Ok() not used, since the two error types are the only information needed.
            }
            info!("Automatic keepalive thread ended.");
        });
        handle
    }

    /// Returns the version of the firmware.
    /// 
    /// If available, this returns the firmware version of the card.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use log::{info, error};
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// let result = wd.get_firmware_version();
    /// match result{
    ///    Ok(fw_ver) => info!("Firmware version:{}", fw_ver),
    ///    Err(errno) => {
    ///        error!("error:{}", errno);
    ///    },
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_firmware_version(&self) -> Result<u32, Errno> {
        #[cfg(unix)]
        let mut wd_info: watchdog_info = watchdog_info::new();
        let result;
        unsafe{
            result = ioctl_get_support(self.file.as_raw_fd(),
                                       &mut wd_info as *mut watchdog_info);
        }
        match result{
            Ok(_) => Ok(wd_info.firmware_version),
            Err(e) => Err(e),
        }
    }

    fn bitmask_query(&self, option: &OptionFlags, query: &BitmaskQueryType) -> Result<bool, Errno> {
        #[cfg(unix)]
        let mut bitmask: c_int = -1;
        let result;
        match query{
            BitmaskQueryType::GetStatus =>{
                unsafe{
                    result = ioctl_get_status(self.file.as_raw_fd(),
                                         &mut bitmask as *mut c_int);
                }
            }
            BitmaskQueryType::GetBootStatus =>{
                unsafe{
                    result = ioctl_get_bootstatus(self.file.as_raw_fd(),
                                             &mut bitmask as *mut c_int);
                }
            }
        }
        match result{
            Ok(_) => {
                trace!("bitmask: \n{:#034b}\n{:#034b}", 
                         option.value(),
                         bitmask);
                Ok((bitmask as u32 & option.value()) != 0)
            },
            Err(e) => Err(e),
        }
    }

    /// Returns the status of an option.
    /// 
    /// For any supported option (see [`is_option_supported()`](Self::is_option_supported)), this returns its related current status.
    /// See also [`get_boot_status()`](Self::get_boot_status) to retrieve the status at the last reboot.
    /// Note that not all devices support these two calls; some only support one of them.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// let option = OptionFlags::Overheat;
    /// if wd.is_option_supported(&option).unwrap(){
    ///     info!("Overheat:{}", wd.get_status(&option).unwrap());
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_status(&self, option: &OptionFlags) -> Result<bool, Errno> {
        self.bitmask_query(option, &BitmaskQueryType::GetStatus)
    }

    /// Returns the status of an option at the last reboot.
    /// 
    /// For any supported option (see [`is_option_supported()`](Self::is_option_supported)), this returns its related status at the last reboot.
    /// See also [`get_status()`](Self::get_status) to retrieve the current status.
    /// Note that not all devices support these two calls; some only support one of them.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// let option = OptionFlags::Overheat;
    /// if wd.is_option_supported(&option).unwrap(){
    ///     info!("Overheat at last boot:{}", wd.get_boot_status(&option).unwrap());
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_boot_status(&self, option: &OptionFlags) -> Result<bool, Errno> {
        self.bitmask_query(option, &BitmaskQueryType::GetBootStatus)
    }

    /// Tells if an option is supported.
    /// 
    /// From the Linux Kernel Watchdog API documentation:
    /// 
    /// All watchdog drivers are required return more information about the system, some do temperature, fan and power level monitoring, 
    /// some can tell you the reason for the last reboot of the system. 
    /// 
    /// This call is available to ask what the device can do. See [`OptionFlags`] for a description of each option.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// info!("FanFault option support:{}", wd.is_option_supported(&OptionFlags::FanFault).unwrap());
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_option_supported(&self, option: &OptionFlags) -> Result<bool, Errno> {
        #[cfg(unix)]
        let mut wd_info: watchdog_info = watchdog_info::new();
        let result;
        unsafe{
            result = ioctl_get_support(self.file.as_raw_fd(),
                                       &mut wd_info as *mut watchdog_info);
        }
        match result{
            Ok(_) => {
                trace!("options bitmask: \n{:#034b}\n{:#034b}", 
                         option.value(),
                         wd_info.options);
                Ok((wd_info.options & option.value()) != 0)
            },
            Err(e) => Err(e),
        }
    }

    /// Returns the watchdog driver identifier.
    /// 
    /// This returns a String containing the identifier for the watchdog driver.
    ///
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// info!("Driver ID:{}", wd.get_driver_identity().unwrap());
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_driver_identity(&self) -> Result<String, Errno> {
        #[cfg(unix)]
        let mut wd_info: watchdog_info = watchdog_info::new();
        let result;
        unsafe{
            result = ioctl_get_support(self.file.as_raw_fd(),
                                       &mut wd_info as *mut watchdog_info);
        }
        // allocate new string containing the converted u8 array.
        let string_ident = String::from_utf8_lossy(&wd_info.identity).into_owned();
        match result{
            Ok(_) => Ok(string_ident),
            Err(e) => Err(e),
        }
    }

    fn int_getter(&self, getter_type: IntGetterType) -> Result<i32, Errno> {
        #[cfg(unix)]
        let mut value: c_int = -1;
        let result = match getter_type{
            IntGetterType::GetTimeout => unsafe{
                ioctl_get_timeout(self.file.as_raw_fd(), &mut value as *mut c_int)
            },
            IntGetterType::GetPreTimeout => unsafe{
                ioctl_get_pretimeout(self.file.as_raw_fd(), &mut value as *mut c_int)
            },
            IntGetterType::GetTimeLeft => unsafe{
                ioctl_get_time_left(self.file.as_raw_fd(), &mut value as *mut c_int)
            },
            IntGetterType::GetTemp => unsafe{
                ioctl_get_temp(self.file.as_raw_fd(), &mut value as *mut c_int)
            },
        };
        match result{
            Ok(_) => Ok(value),
            Err(e) => Err(e),
        }
    }

    /// Returns the configured timeout.
    /// 
    /// This is used to know the number of seconds the watchdog will wait before triggering a reset.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// info!("Current configured timeout:{}", wd.get_timeout().unwrap());
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_timeout(&self) -> Result<i32, Errno> {
        self.int_getter(IntGetterType::GetTimeout)
    }

    /// Returns the configured pre-timeout, if suppported.
    /// 
    /// From the Linux Kernel Watchdog API documentation:
    /// 
    /// Some watchdog timers can be set to have a trigger go off before the actual time they will reset the system. 
    /// This can be done with an NMI, interrupt, or other mechanism. 
    /// This allows Linux to record useful information (like panic information and kernel coredumps) before it resets.
    /// 
    /// Note that the pretimeout is the number of seconds before the time when the timeout will go off. 
    /// It is not the number of seconds until the pretimeout. 
    /// So, for instance, if you set the timeout to 60 seconds and the pretimeout to 10 seconds, 
    /// the pretimeout will go off in 50 seconds. Setting a pretimeout to zero disables it.
    /// 
    /// Not all watchdog drivers will support a pretimeout.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// if wd.is_option_supported(&OptionFlags::PreTimeout).unwrap(){
    ///     info!("Current configured pre-timeout:{}", wd.get_timeout().unwrap());
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_pretimeout(&self) -> Result<i32, Errno> {
        self.int_getter(IntGetterType::GetPreTimeout)
    }

    /// Returns the time left before reset.
    /// 
    /// Some watchdog drivers have the ability to report the remaining time before the system will reboot.
    /// The returned value is the number of seconds left at the moment of the call.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// info!("Number of seconds left:{}", wd.get_time_left().unwrap());
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_time_left(&self) -> Result<i32, Errno> {
        self.int_getter(IntGetterType::GetTimeLeft)
    }

    /// Returns the current temperature.
    /// 
    /// Some drivers can measure the temperature. 
    /// The returned value is the temperature in degrees fahrenheit.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::Watchdog;
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// info!("Current temperature:{}F", wd.get_temp().unwrap());
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_temp(&self) -> Result<i32, Errno> {
        self.int_getter(IntGetterType::GetTemp)
    }

    /// Configures the timeout, if supported.
    ///  
    /// For some drivers it is possible to modify the watchdog timeout on the fly by calling this method. 
    /// It is possible to verify the support for this feature by calling [`is_option_supported()`](Self::is_option_supported) with [`OptionFlags::SetTimeout`] as an argument. 
    /// The argument is an integer representing the timeout in seconds. If the value is unsupported, the function will return an EINVAL error.
    /// The driver returns the real timeout used in the same variable, and this timeout might differ from the requested one due to limitation of the hardware
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// if wd.is_option_supported(&OptionFlags::SetTimeout).unwrap(){
    ///     let requested_timeout = 14;
    ///     let returned_configured_timeout = wd.set_timeout(requested_timeout).unwrap();
    ///     info!("requested timeout:{} - returned value:{}", requested_timeout, returned_configured_timeout);
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_timeout(&self, timeout: i32) -> Result<i32, Errno> {
        #[cfg(unix)]
        let mut timeout_for_ioctl: c_int = timeout;
        let result;
        unsafe{
            result = ioctl_set_timeout(self.file.as_raw_fd(), 
                                       &mut timeout_for_ioctl as *mut c_int);
        }
        match result{
            Ok(_) => Ok(timeout_for_ioctl),
            Err(e) => Err(e),
        }
    }

    /// Configures the pre-timeout, if suppported.
    /// 
    /// From the Linux Kernel Watchdog API documentation:
    /// 
    /// Some watchdog timers can be set to have a trigger go off before the actual time they will reset the system. 
    /// This can be done with an NMI, interrupt, or other mechanism. 
    /// This allows Linux to record useful information (like panic information and kernel coredumps) before it resets.
    /// 
    /// Note that the pretimeout is the number of seconds before the time when the timeout will go off. 
    /// It is not the number of seconds until the pretimeout. 
    /// So, for instance, if you set the timeout to 60 seconds and the pretimeout to 10 seconds, 
    /// the pretimeout will go off in 50 seconds. Setting a pretimeout to zero disables it.
    /// 
    /// The argument is an integer representing the timeout in seconds. 
    /// The driver returns the real timeout used in the same variable, 
    /// and this timeout might differ from the requested one due to limitation of the hardware.
    /// 
    /// Not all watchdog drivers will support a pretimeout.
    /// 
    /// # Examples
    /// 
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// use log::info;
    /// 
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// if wd.is_option_supported(&OptionFlags::PreTimeout).unwrap(){
    ///     let requested_timeout = 27;
    ///     let returned_configured_timeout = wd.set_pretimeout(requested_timeout).unwrap();
    ///     info!("requested pre-timeout:{} - returned value:{}", requested_timeout, returned_configured_timeout);
    /// }
    /// # wd.magic_close()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_pretimeout(&self, pretimeout: i32) -> Result<i32, Errno> {
        #[cfg(unix)]
        let mut pretimeout_for_ioctl: c_int = pretimeout;
        let result;
        unsafe{
            result = ioctl_set_pretimeout(self.file.as_raw_fd(), 
                                     &mut pretimeout_for_ioctl as *mut c_int);
        }
        match result{
            Ok(_) => Ok(pretimeout_for_ioctl),
            Err(e) => Err(e),
        }
    }

    /// Sets a watchdog operation.
    /// 
    /// This can be used to control some aspects of the card operation, if supported.
    /// The [`SetOptionFlags`] enum lists all the operations that is possible to trigger.
    pub fn set_option(&self, option: &SetOptionFlags) -> Result<(), Errno> {
        #[cfg(unix)]
        let mut option_to_set: c_int = 
            option.value().try_into().expect("option not convertible to c_int");
        let result;
        unsafe{
            result = ioctl_set_options(self.file.as_raw_fd(), 
                                       &mut option_to_set as *mut c_int);
        }
        match result{
            Ok(res) => {trace!("Set_option {} returned {}.", option, res); Ok(())},
            Err(e) => Err(e),
        }
    }

    /// Disables the watchdog, if supported.
    /// 
    /// If a driver supports “Magic Close”, the driver will not disable the watchdog unless [`magic_close()`](Self::magic_close) is called 
    /// just before releasing the [`Watchdog`] instance. 
    /// If the user closes the watchdog without calling this, the driver will assume that the program (and userspace in general) died, 
    /// and will stop pinging the watchdog without disabling it first. This will then cause a reboot if the watchdog is not re-opened in sufficient time.
    /// 
    /// When the device is closed, the watchdog is disabled, unless the “Magic Close” feature is supported (see below). 
    /// This is not always such a good idea, since if there is a bug in the watchdog daemon and it crashes the system will not reboot. 
    /// Because of this, some of the drivers support the configuration option “Disable watchdog shutdown on close”, CONFIG_WATCHDOG_NOWAYOUT. 
    /// If it is set to Y when compiling the kernel, there is no way of disabling the watchdog once it has been started. 
    /// So, if the watchdog daemon crashes, the system will reboot after the timeout has passed. 
    /// Watchdog devices also usually support the nowayout module parameter so that this option can be controlled at runtime.
    /// 
    /// # Examples
    ///
    /// ```rust
    /// use watchdog_device::{Watchdog, OptionFlags};
    /// use nix::errno::Errno;
    /// 
    /// # fn do_something(){}
    /// # fn main() -> Result<(), std::io::Error> {
    /// let mut wd = Watchdog::new()?;
    /// # let mut keep_running = true;
    /// while keep_running{
    ///     do_something();
    ///     if let Err(e) = wd.keep_alive(){
    ///         println!("Error {}", e);
    ///     }
    /// #   keep_running = false;
    /// }
    /// if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
    ///     wd.magic_close()?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn magic_close(&mut self) -> std::io::Result<()>{
        // If the automatic keepalive thread is running, send signal to close thread...
        if self.msg_sender.is_some(){
            // Drop sender, to let the receiver understand it must exit.
            self.msg_sender = None;
        }

        self.file.write_all(b"V")?;
        self.file.flush()?;
        warn!("Magic close. The watchdog will NOT restart the system.");
        Ok(())
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        warn!("Closing watchdog file...");
    }
}
