
macro_rules! warn {
    ($($args:tt)*) => {{
        use crossterm::{Colorize, Styler};
        eprint!("{}: ", "Warning".bold().yellow());
        eprintln!($($args)*);
    }};
}

macro_rules! error {
    ($($args:tt)*) => {{
        use crossterm::{Colorize, Styler};
        eprint!("{}: ", "Error".bold().red());
        eprintln!($($args)*);
    }};
}
