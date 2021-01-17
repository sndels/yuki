#[macro_export]
macro_rules! expect {
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(t) => t,
            Err(why) => {
                panic!("{}: {:?}", $msg, why);
            }
        }
    };
}

#[macro_export]
macro_rules! error {
    ($msg:expr) => {
        let debug_msg = format!("{}\n", $msg);
        win_dbg_logger::output_debug_string(&debug_msg);
        log::error!("{}", $msg);
    };
}
