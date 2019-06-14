macro_rules! stderr_style {
    ([$($attr:expr),*], { $($cmd:tt)+ }) => {
        let mut term = term::stderr().unwrap();
        $(
            let _ = term.attr($attr);
        )+
        $($cmd)+;
        let _ = term.reset();
    };
}

macro_rules! warn {
    ($($args:tt)*) => {{
        stderr_style!(
            [
                term::Attr::ForegroundColor(term::color::YELLOW),
                term::Attr::Bold
            ],
            { 
                eprint!("Warning: ");
            }
        );
        eprintln!($($args)*);
    }}
}

macro_rules! error {
    ($($args:tt)*) => {
        stderr_style!(
            [
                term::Attr::ForegroundColor(term::color::RED),
                term::Attr::Bold
            ],
            {
                eprint!("Error: ");
            }
        );
        eprintln!($($args)*);
    }
}
