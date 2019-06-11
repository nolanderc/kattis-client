
macro_rules! warn {
    ($msg:expr $(, $args:tt)*) => {
        eprintln!(concat!("Warning: ", $msg) $(, $args)*)
    }
}

