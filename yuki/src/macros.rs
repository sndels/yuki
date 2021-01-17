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
