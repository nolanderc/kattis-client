
macro_rules! warn {
    ($($args:tt)*) => {{
        let mut term = term::stderr().unwrap();
        term.fg(term::color::YELLOW).unwrap();
        term.attr(term::Attr::Bold).unwrap();
        eprint!("Warning: ");
        term.reset().unwrap();
        eprintln!($($args)*);
    }}
}

macro_rules! error {
    ($($args:tt)*) => {
        let mut term = term::stderr().unwrap();
        term.fg(term::color::RED).unwrap();
        term.attr(term::Attr::Bold).unwrap();
        eprint!("Error: ");
        term.reset().unwrap();
        eprintln!($($args)*);
    }
}

