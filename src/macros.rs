macro_rules! log {
    ($is_verbose:ident, $message:expr) => {
        if $is_verbose.verbose {
            print!($message);
        }
    };

    ($is_verbose:ident, $message:expr, $($args:expr),*) => {
        if $is_verbose.verbose {
            print!($message $(,$args)*);
        }
    };
}

macro_rules! logln {
    ($is_verbose:ident, $message:expr) => {
        if $is_verbose.verbose {
            println!($message);
        }
    };

    ($is_verbose:ident, $message:expr, $($args:expr),*) => {
        if $is_verbose.verbose {
            println!($message $(,$args)*);
        }
    };
}
