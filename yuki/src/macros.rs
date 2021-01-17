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
/// Takes in a format string
macro_rules! yuki_error {
    ( $( $arg:expr ),+ ) => {
        let msg = format!( $( $arg ),+ );
        win_dbg_logger::output_debug_string(&msg);
        log::error!("{}", &msg);
    };
}

#[macro_export]
/// Takes in a format string
macro_rules! yuki_debug {
    ( $( $arg:expr ),+ ) => {
        log::debug!( $( $arg ),+ );
    };
}
