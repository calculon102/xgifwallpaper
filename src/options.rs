//! Defines options of `xgifwallpaper` and parses these from command line-
//! arguments.

use clap::{value_t, App, Arg, ArgMatches};

use super::position::Scaling;
use super::EXIT_INVALID_DELAY;
use super::VERSION;

const ARG_COLOR: &str = "COLOR";
const ARG_DELAY: &str = "DELAY";
const ARG_PATH_TO_GIF: &str = "PATH_TO_GIF";
const ARG_SCALE: &str = "SCALE";
const ARG_VERBOSE: &str = "VERBOSE";

/// Runtime options as given by the caller of this program.
pub struct Options {
    pub background_color: String,
    pub default_delay: u16,
    pub path_to_gif: String,
    pub scaling: Scaling,
    pub verbose: bool,
}

impl Options {
    /// Parse options from command-line.
    pub fn from_args() -> Options {
        parse_args(init_args().get_matches())
    }

    fn _from_params(params: Vec<&str>) -> Options {
        parse_args(init_args().get_matches_from(params))
    }
}

/// Declare command-line-arguments.
fn init_args<'a, 'b>() -> App<'a, 'b> {
    App::new("xgifwallpaper")
        .version(VERSION)
        .author("Frank Großgasteiger <frank@grossgasteiger.de>")
        .about("Animates a GIF as wallpaper in your X-session")
        .arg(
            Arg::with_name(ARG_COLOR)
                .short("b")
                .long("background-color")
                .takes_value(true)
                .value_name("X11-color")
                .default_value("#000000")
                .help("X11 compilant color-name to paint background."),
        )
        .arg(
            Arg::with_name(ARG_DELAY)
                .short("d")
                .long("default-delay")
                .takes_value(true)
                .value_name("default-delay")
                .default_value("10")
                .help("Delay in centiseconds between frames, if unspecified in GIF."),
        )
        .arg(Arg::with_name(ARG_VERBOSE).short("v").help("Verbose mode"))
        .arg(
            Arg::with_name(ARG_PATH_TO_GIF)
                .help("Path to GIF-file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name(ARG_SCALE)
                .short("s")
                .long("scale")
                .takes_value(true)
                .possible_values(&["NONE", "FILL", "MAX"])
                .default_value("NONE")
                .help("Scale GIF-frames, relative to available screen."),
        )
}

/// Parse arguments from command line.
fn parse_args<'a>(args: ArgMatches<'a>) -> Options {
    let delay = value_t!(args, ARG_DELAY, u16).unwrap_or_else(|_e| {
        eprintln!(
            "Use a value between {} and {} as default-delay.",
            u16::MIN,
            u16::MAX
        );
        std::process::exit(EXIT_INVALID_DELAY)
    });

    let scaling = match args.value_of(ARG_SCALE).unwrap() {
        "NONE" => Scaling::NONE,
        "FILL" => Scaling::FILL,
        "MAX" => Scaling::MAX,
        &_ => Scaling::NONE, // Cannot happen, due to guarantee of args
    };

    Options {
        background_color: args.value_of(ARG_COLOR).unwrap().to_owned(),
        default_delay: delay,
        path_to_gif: args.value_of(ARG_PATH_TO_GIF).unwrap().to_owned(),
        scaling,
        verbose: args.is_present(ARG_VERBOSE),
    }
}

#[cfg(test)]
mod tests {
    use super::Options;
    use super::Scaling;

    #[test]
    fn when_argument_scale_is_none_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "NONE"]));
        assert_eq!(options.scaling, Scaling::NONE);
    }

    #[test]
    fn when_argument_scale_is_fill_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "FILL"]));
        assert_eq!(options.scaling, Scaling::FILL);
    }

    #[test]
    fn when_argument_scale_is_max_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "MAX"]));
        assert_eq!(options.scaling, Scaling::MAX);
    }

    fn _create_params(custom_params: Vec<&str>) -> Vec<&str> {
        [vec!["xgifwallpaper"], custom_params, vec!["wallpaper.gif"]].concat()
    }
}
