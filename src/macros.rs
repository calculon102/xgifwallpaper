//! Custom macros for `xgifwallpaper`.

/// Prints message, if verbose is `true` in given `Options`, without implicit
/// line-break.
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate xgifwallpaper;
/// # fn main() {
/// # use xgifwallpaper::options::Options;
/// let options = Options::_from_params(vec!["xgifwallpaper", "-v", "foobar.gif"]);
/// assert_eq!(options.verbose, true);
///
/// log!(options, "This should be logged ... ");
/// log!(options, "on the same line.");
/// # }
/// ```
///
/// ```
/// # #[macro_use] extern crate xgifwallpaper;
/// # fn main() {
/// # use xgifwallpaper::options::Options;
/// let options = Options::_from_params(vec!["xgifwallpaper", "foobar.gif"]);
/// assert_eq!(options.verbose, false);
///
/// log!(options, "This will not be logged ... ");
/// log!(options, "in no way.");
/// # }
/// ```
#[macro_export]
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

/// Prints message, if verbose is `true` in given `Options`, without implicit
/// line-break.
///
/// # Examples
///
/// ```
/// # #[macro_use] extern crate xgifwallpaper;
/// # fn main() {
/// # use xgifwallpaper::options::Options;
/// let options = Options::_from_params(vec!["xgifwallpaper", "-v", "foobar.gif"]);
/// assert_eq!(options.verbose, true);
///
/// logln!(options, "This should be logged ... ");
/// logln!(options, "on two lines.");
/// # }
/// ```
///
/// ```
/// # #[macro_use] extern crate xgifwallpaper;
/// # fn main() {
/// # use xgifwallpaper::options::Options;
/// let options = Options::_from_params(vec!["xgifwallpaper", "foobar.gif"]);
/// assert_eq!(options.verbose, false);
///
/// logln!(options, "This will not be logged ... ");
/// logln!(options, "and never on two lines.");
/// # }
/// ```
#[macro_export]
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
