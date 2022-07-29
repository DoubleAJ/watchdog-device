#[cfg(test)]
mod tests {
    use watchdog_device::{Watchdog, OptionFlags, SetOptionFlags};
    use log::{error, warn, info, trace};
    use std::time::Duration;
    use std::thread::sleep;
    use std::sync::{Arc, Mutex, Once};
    
    static INIT: Once = Once::new();

    #[cfg(test)]
    fn init_logger(){
        INIT.call_once(|| {
            let _ = env_logger::builder().is_test(true).try_init();
        });
    }

    #[test]
    fn test_open() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        Ok(())
    }

    #[test]
    fn test_keep_alive() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let timeout = wd.get_timeout().unwrap();
        let mut result = Ok(());
        // Tries to send one keep alive per second for twice the duration of the timeout,
        // in order to verify that no reset is triggered.
        info!("Timeout is {} seconds, so keep alive signals will be sent 
                    once every second for twice as long.", timeout);
        for counter in 0..2*timeout{
            result = wd.keep_alive();
            match result{
                Ok(_) => info!("Keep alive #{} sent.", counter),
                Err(e) => {
                    error!("Keep alive #{} failed with error:{}", counter, e);
                    break;
                }
            }    
            sleep(Duration::from_secs(1));
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(result.is_err(), false);
        Ok(())
    }

    #[test]
    fn test_magic_close() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let option = OptionFlags::MagicClose;
        if wd.is_option_supported(&option)?
        {
            wd.magic_close()?;
        }
        else {
            warn!("Option {} is not supported", option);
        }
        Ok(())
    }

    #[test]
    fn test_get_firmware_version() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let result = wd.get_firmware_version();
        match result{
            Ok(fw_ver) => info!("Firmware ver:{}", fw_ver),
            Err(errno) => {
                error!("error:{}", errno);
            },
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(result.is_err(), false);
        Ok(())
    }

    #[test]
    fn test_get_option_flags() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        // all the enum variants:
        let options = vec![
            OptionFlags::Overheat,       
            OptionFlags::FanFault,       
            OptionFlags::Extern1,        
            OptionFlags::Extern2,        
            OptionFlags::PowerUnder,     
            OptionFlags::CardReset,     
            OptionFlags::PowerOver,      
            OptionFlags::SetTimeout,     
            OptionFlags::MagicClose,     
            OptionFlags::PreTimeout,     
            OptionFlags::AlarmOnly,      
            OptionFlags::KeepalivePing
        ];
        let mut test_error = false;
        for option in options{
            let result = wd.is_option_supported(&option);
            match result{
                Ok(opt_res) => info!("option {}:{}", option, opt_res),
                Err(errno) => {
                    error!("error:{}", errno);
                    test_error = true;
                    break;
                },
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    #[test]
    fn test_get_driver_identity() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let result = wd.get_driver_identity();
        match result{
            Ok(ref identity) => info!("driver identity:{}", identity),
            Err(errno) => {
                error!("error:{}", errno);
            },
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(result.is_err(), false);
        Ok(())
    }

    #[test]
    fn test_get_timeout() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let result = wd.get_timeout();
        match result{
            Ok(timeout) => info!("timeout:{} secs", timeout),
            Err(errno) => {
                error!("error. errno:{}", errno);
            },
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(result.is_err(), false);
        Ok(())
    }

    #[test]
    fn test_get_pretimeout() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let mut test_error = false;
        let option = OptionFlags::PreTimeout;
        match wd.is_option_supported(&option){
            Ok(supported) => {
                if supported{
                    match wd.get_pretimeout(){
                        Ok(timeout) => info!("pretimeout:{} secs", timeout),
                        Err(errno) => {
                            test_error = true;
                            error!("error getting pretimeout. errno:{}", errno);
                        },
                    }
                }
                else{
                    warn!("Option {} is not supported", option);
                }
            }
            Err(e) => {
                test_error = true;
                error!("error checking if option was supported: {}", e);
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert!(!test_error);
        Ok(())
    }

    #[test]
    fn test_get_time_left() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        for _ in 0..3{
            let result = wd.get_time_left();
            match result{
                Ok(time_left) => info!("time left:{} secs", time_left),
                Err(errno) => {
                    error!("error. errno:{}", errno);
                    assert!(false); // flag this test as failed.
                    break;
                },
            }
            sleep(Duration::from_secs(1));
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        Ok(())
    }

    #[test]
    fn test_get_temp() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let result = wd.get_temp();
        match result{
            Ok(temp) => info!("temperature:{}F", temp),
            Err(errno) => {
                // Do not let the test fail, since it is possible that the card simply doesn't support this feature.
                // There is no other way of knowing if it is supported; 
                // there is no related option flag in "is_option_supported(OptionFlags)".
                warn!("Couldn't get temperature. errno:{}", errno);
            },
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        Ok(())
    }

    // From https://www.kernel.org/doc/html/latest/watchdog/watchdog-api.html
    // GET_STATUS and GET_BOOT_STATUS can return information about the following options (if supported):
    // Overheat
    // fanfault
    // extern1
    // extern2
    // powerunder
    // cardreset
    // powerover
    // keepaliveping

    #[test]
    fn test_get_status() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        // all the enum variants:
        let options = vec![
            OptionFlags::Overheat,       
            OptionFlags::FanFault,       
            OptionFlags::Extern1,        
            OptionFlags::Extern2,        
            OptionFlags::PowerUnder,     
            OptionFlags::CardReset,     
            OptionFlags::PowerOver,      
            OptionFlags::KeepalivePing
        ];
        let mut test_error = false;
        for option in options{
            let result = wd.get_status(&option);
            match result{
                Ok(status) => info!("status {}:{}", option, status),
                Err(errno) => {
                    error!("error:{}", errno);
                    test_error = true;
                    break;
                },
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    #[test]
    fn test_get_boot_status() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        // all the enum variants:
        let options = vec![
            OptionFlags::Overheat,       
            OptionFlags::FanFault,       
            OptionFlags::Extern1,        
            OptionFlags::Extern2,        
            OptionFlags::PowerUnder,     
            OptionFlags::CardReset,     
            OptionFlags::PowerOver,      
            OptionFlags::KeepalivePing
        ];
        let mut test_error = false;
        for option in options{
            let result = wd.get_boot_status(&option);
            match result{
                Ok(status) => info!("boot status {}:{}", option, status),
                Err(errno) => {
                    error!("error:{}", errno);
                    test_error = true;
                    break;
                },
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    #[test]
    fn test_set_timeout() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let option = OptionFlags::SetTimeout;
        let mut test_error = false;
        match wd.is_option_supported(&option){
            Ok(supported) => {
                if supported{
                    let original_timeout_val = wd.get_timeout().unwrap();
                    let modified_timeout_val = original_timeout_val - 1;
                    let mut returned_timeout = -1;
                    match wd.set_timeout(modified_timeout_val){
                        Ok(rt) => returned_timeout = rt,
                        Err(e) => {                        
                            error!("error:{}", e);
                            test_error = true;
                        },
                    }
                    let new_timeout = wd.get_timeout().unwrap();
                    if (!test_error) && (new_timeout != returned_timeout){
                        test_error = true;
                        error!("The timeout returned from set_timeout():{} is different from get_timeout(){}.", 
                            returned_timeout, new_timeout);
                    }
                    // Restore original value
                    match wd.set_timeout(original_timeout_val){
                        Ok(rt) => returned_timeout = rt,
                        Err(e) => {                        
                            error!("error:{}", e);
                            test_error = true;
                        },
                    }
                    if (!test_error) && (original_timeout_val != returned_timeout){
                        test_error = true;
                        error!("The timeout returned from set_timeout():{} is different from the original timeout:{}.", 
                            returned_timeout, original_timeout_val);
                    }
                }
                else{
                    warn!("Option {} is not supported", option);
                }
            }
            Err(e) => {
                test_error = true;
                error!("error checking if option was supported: {}", e);
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    #[test]
    fn test_set_pretimeout() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let option = OptionFlags::PreTimeout;
        let mut test_error = false;
        match wd.is_option_supported(&option){
            Ok(supported) => {
                if supported{
                    let original_timeout_val = wd.get_pretimeout().unwrap();
                    let modified_timeout_val = original_timeout_val - 1;
                    let mut returned_timeout = -1;
                    match wd.set_pretimeout(modified_timeout_val){
                        Ok(rt) => returned_timeout = rt,
                        Err(e) => {                        
                            error!("error:{}", e);
                            test_error = true;
                        },
                    }
                    let new_timeout = wd.get_pretimeout().unwrap();
                    if (!test_error) && (new_timeout != returned_timeout){
                        test_error = true;
                        error!("The timeout returned from set_pretimeout():{} is different from get_pretimeout(){}.", 
                            returned_timeout, new_timeout);
                    }
                    // Restore original value
                    match wd.set_pretimeout(original_timeout_val){
                        Ok(rt) => returned_timeout = rt,
                        Err(e) => {                        
                            error!("error:{}", e);
                            test_error = true;
                        },
                    }
                    if (!test_error) && (original_timeout_val != returned_timeout){
                        test_error = true;
                        error!("The timeout returned from set_pretimeout():{} is different from the original pretimeout:{}.", 
                            returned_timeout, original_timeout_val);
                    }
                }
                else{
                    warn!("Option {} is not supported", option);
                }
            }
            Err(e) => {
                test_error = true;
                error!("error checking if option was supported: {}", e);
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    // This test is ignored to avoid altering the current card configuration. 
    // It can of course be activated and tried manually, but it is ignored by default.
    #[ignore]
    #[test]
    fn test_set_options() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        // all the enum variants:
        let options = vec![
            SetOptionFlags::DisableCard,       
            SetOptionFlags::EnableCard,       
            SetOptionFlags::TempPanic,       
        ];
        let mut test_error = false;
        for option in options{
            let result = wd.set_option(&option);
            match result{
                Ok(_) => info!("Option {} set correctly.", option),
                Err(errno) => {
                    error!("error:{}", errno);
                    test_error = true;
                    break;
                },
            }
        }
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        assert_eq!(test_error, false);
        Ok(())
    }

    // This test is ignored to avoid altering the current card configuration. 
    // It can of course be activated and tried manually, but it is ignored by default.
    #[ignore]
    #[test]
    fn test_disable_card() -> Result<(), std::io::Error> {
        init_logger();
        let wd = Watchdog::new()?;
        let result = wd.set_option(&SetOptionFlags::DisableCard);
        match result{
            Ok(_) => info!("Card disabled correctly."),
            Err(errno) => {
                error!("error:{}", errno);
            },
        }
        assert_eq!(result.is_err(), false);
        //wd.magic_close()?; // No magic close, in order to verify if the watchdog is actually disabled.
        trace!("Test over. The watchdog is disabled, so the system shouldn't reset.");
        Ok(())
    }

    // This test is ignored to avoid altering the current card configuration. 
    // It can of course be activated and tried manually, but it is ignored by default.
    #[ignore]
    #[test]
    fn test_enable_card() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd = Watchdog::new()?;
        let result = wd.set_option(&SetOptionFlags::EnableCard);
        match result{
            Ok(_) => info!("Card enabled correctly."),
            Err(errno) => {
                error!("error:{}", errno);
            },
        }
        assert_eq!(result.is_err(), false);
        if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd.magic_close()?;
        }
        Ok(())
    }

    #[test]
    fn test_automatic_keepalive() -> Result<(), std::io::Error> {
        init_logger();
        let wd = Watchdog::new()?;
        let wd_mutex_arc: Arc<Mutex<Watchdog>> = Arc::new(Mutex::new(wd));
        let handle = Watchdog::start_automatic_keep_alive(wd_mutex_arc.clone());

        let mut wait_duration: u64 = 45; // By default the test will try to wait longer than a theoretical timeout delay.
        if let Ok(timeout) = wd_mutex_arc.lock().expect("Mutex poisoned while getting timeout.").get_timeout(){
            wait_duration = (timeout * 2) as u64;
        }
        info!("Sleeping for {} secs to verify that the watchdog won't restart the system...", wait_duration);
        sleep(Duration::from_secs(wait_duration));

        {
            let locked_wd = &mut *wd_mutex_arc.lock().expect("Error obtaining lock guard.");
            if locked_wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
                locked_wd.magic_close()?;
            }
        }
        handle.join().expect("Error joining thread.");
        Ok(())
    }

    // This test is disabled because it is used to verify that the system actually resets 
    // when no magic_close is used before releasing the watchdog instance.
    #[ignore]
    #[test]
    fn test_automatic_keepalive_no_magic_close() -> Result<(), std::io::Error> {
        init_logger();
        let wd = Watchdog::new()?;
        let wd_mutex_arc: Arc<Mutex<Watchdog>> = Arc::new(Mutex::new(wd));
        let _handle = Watchdog::start_automatic_keep_alive(wd_mutex_arc.clone());

        let mut wait_duration: u64 = 45; // By default the test will try to wait longer than a theoretical timeout delay.
        if let Ok(timeout) = wd_mutex_arc.lock().expect("Mutex poisoned while getting timeout.").get_timeout(){
            wait_duration = (timeout * 2) as u64;
        }
        info!("Sleeping for {} secs to verify that the watchdog won't restart the system...", wait_duration);
        sleep(Duration::from_secs(wait_duration));
        
        // No Magic Close; the system should reset!

        // {
        //     let locked_wd = &mut *wd_mutex_arc.lock().expect("Error obtaining lock guard.");
        //     locked_wd.magic_close()?;
        // }
        // handle.join().expect("Error joining thread.");
        Ok(())
    }

    #[test]
    fn test_multiple_instances() -> Result<(), std::io::Error> {
        init_logger();
        let mut wd1 = Watchdog::new()?;
        let res = Watchdog::new();
        assert_eq!(res.is_err(), true); // Should fail
        if wd1.is_option_supported(&OptionFlags::MagicClose).unwrap(){
            wd1.magic_close()?;
        }
        Ok(())
    }

    #[test]
    fn test_successive_opening() -> Result<(), std::io::Error> {
        // Test opening and closign several times.
        init_logger();
        {
            let _wd = Watchdog::new()?;
        }
        {
            let _wd = Watchdog::new()?;
        }
        {
            let mut wd = Watchdog::new()?;
            if wd.is_option_supported(&OptionFlags::MagicClose).unwrap(){
                wd.magic_close()?;
            }
        }
        Ok(())
    }

}
