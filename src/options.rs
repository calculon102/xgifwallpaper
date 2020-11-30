//! Defines options of `xgifwallpaper` and parses these from command line-
//! arguments.

use clap::{value_t, App, Arg, ArgMatches};

use super::position::Scaling;
use super::position::ScalingFilter;
use super::VERSION;

const ARG_COLOR: &str = "COLOR";
const ARG_DELAY: &str = "DELAY";
const ARG_PATH_TO_GIF: &str = "PATH_TO_GIF";
const ARG_SCALE: &str = "SCALE";
const ARG_SCALE_FILTER: &str = "SCALE_FILTER";
const ARG_VERBOSE: &str = "VERBOSE";
const ARG_WINDOW_ID: &str = "WINDOW_ID";

const DEFAULT_DELAY: u16 = 10;
const DEFAULT_DELAY_STR: &str = "10";

/// Runtime options as given by the caller of this program.
pub struct Options {
    /// X11-compilant color-name
    pub background_color: String,
    pub default_delay: u16,
    pub path_to_gif: String,
    /// Scaling-method to use
    pub scaling: Scaling,
    pub scaling_filter: ScalingFilter,
    pub verbose: bool,
    /// Window-Id as decimal or hex-number (0x-prefix) or name of atom with Id
    /// to use.
    pub window_id: String,
}

impl Options {
    /// Parse options from command-line.
    ///
    /// ```
    /// let options = Options::from_args();
    /// ```
    pub fn from_args() -> Options {
        parse_args(init_args().get_matches())
    }

    /// Parse options as strings, in order given.
    ///
    /// ```
    /// let options = Options::_from_params(
    ///     vec!["-v", "-b #FF0000", "foobar.gif"]
    /// );
    ///
    /// assert_eq!(options.background_color, "#FF0000".to_string());
    /// assert_eq!(options.path_to_gif, "foobar.gif".to_string());
    /// assert_eq!(options.verbose, true);
    /// ```
    pub fn _from_params(params: Vec<&str>) -> Options {
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
                .default_value(DEFAULT_DELAY_STR)
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
        .arg(
            Arg::with_name(ARG_SCALE_FILTER)
                .long("scale-filter")
                .takes_value(true)
                .possible_values(&["AUTO", "PIXEL"])
                .default_value("AUTO")
                .help("Filter to use in combination with scale-option. Experimental feature."),
        )
        .arg(
            Arg::with_name(ARG_WINDOW_ID)
                .help(
                    "ID of window to animate wallpaper on its background, \
                    insted of the root window. As decimal, hex or name of \
                    root-atom.",
                )
                .long("window-id")
                .short("w")
                .takes_value(true),
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
        DEFAULT_DELAY
    });

    let scaling = match args.value_of(ARG_SCALE).unwrap() {
        "NONE" => Scaling::NONE,
        "FILL" => Scaling::FILL,
        "MAX" => Scaling::MAX,
        &_ => Scaling::NONE, // Cannot happen, due to guarantee of args
    };

    let scaling_filter = match args.value_of(ARG_SCALE_FILTER).unwrap() {
        "AUTO" => ScalingFilter::AUTO,
        "PIXEL" => ScalingFilter::PIXEL,
        &_ => ScalingFilter::AUTO, // Cannot happen, due to guarantee of args
    };

    Options {
        background_color: args.value_of(ARG_COLOR).unwrap().to_owned(),
        default_delay: delay,
        path_to_gif: args.value_of(ARG_PATH_TO_GIF).unwrap().to_owned(),
        scaling,
        scaling_filter,
        verbose: args.is_present(ARG_VERBOSE),
        window_id: args.value_of(ARG_WINDOW_ID).unwrap_or("").to_string(),
    }
}

// Test only xgifwallpaper-specifics via arguments. Don't test behaviour of
// clap, like mandatory fields, valid options or order of arguments.
#[cfg(test)]
mod tests {
    use super::Options;
    use super::Scaling;
    use super::ScalingFilter;

    const PATH_TO_GIF: &str = "wallpaper.gif";

    #[test]
    fn use_argument_path_to_gif() {
        let options = Options::_from_params(_create_params(vec![]));
        assert_eq!(options.path_to_gif, PATH_TO_GIF);
    }

    #[test]
    fn use_defaults_for_omitted_arguments() {
        let options = Options::_from_params(_create_params(vec![]));
        assert_eq!(options.background_color, "#000000");
        assert_eq!(options.default_delay, 10);
        assert_eq!(options.verbose, false);
        assert_eq!(options.scaling, Scaling::NONE);
        assert_eq!(options.scaling_filter, ScalingFilter::AUTO);
    }

    #[test]
    fn when_argument_default_delay_is_not_u16_then_use_default() {
        // Test arbritary string
        let options = Options::_from_params(_create_params(vec!["-d", "a"]));
        assert_eq!(options.default_delay, 10);

        let min_boundary: i32 = u16::MIN as i32 - 1;
        let min_boundary_str = "\"".to_owned() + &min_boundary.to_string() + "\"";
        let options = Options::_from_params(_create_params(vec!["-d", &min_boundary_str]));
        assert_eq!(options.default_delay, 10);

        let max_boundary: i32 = u16::MAX as i32 + 1;
        let options = Options::_from_params(_create_params(vec!["-d", &max_boundary.to_string()]));
        assert_eq!(options.default_delay, 10);
    }

    #[test]
    fn when_argument_background_color_is_given_then_use_it() {
        let options = Options::_from_params(_create_params(vec!["-b", "white"]));
        assert_eq!(options.background_color, "white");
    }

    #[test]
    fn when_argument_default_delay_is_given_then_use_it() {
        let options = Options::_from_params(_create_params(vec!["-d", "666"]));
        assert_eq!(options.default_delay, 666);
    }

    #[test]
    fn when_argument_verbose_is_given_then_be_it() {
        let options = Options::_from_params(_create_params(vec!["-v"]));
        assert_eq!(options.verbose, true);
    }

    #[test]
    fn when_argument_scale_is_none_then_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "NONE"]));
        assert_eq!(options.scaling, Scaling::NONE);
    }

    #[test]
    fn when_argument_scale_is_fill_then_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "FILL"]));
        assert_eq!(options.scaling, Scaling::FILL);
    }

    #[test]
    fn when_argument_scale_is_max_then_match_enum() {
        let options = Options::_from_params(_create_params(vec!["-s", "MAX"]));
        assert_eq!(options.scaling, Scaling::MAX);
    }

    #[test]
    fn when_argument_scale_filter_is_auto_then_match_enum() {
        let options = Options::_from_params(_create_params(vec!["--scale-filter", "AUTO"]));
        assert_eq!(options.scaling_filter, ScalingFilter::AUTO);
    }

    #[test]
    fn when_argument_scale_filter_is_pixel_then_match_enum() {
        let options = Options::_from_params(_create_params(vec!["--scale-filter", "PIXEL"]));
        assert_eq!(options.scaling_filter, ScalingFilter::PIXEL);
    }

    #[test]
    fn when_argument_window_id_is_given_then_use_it() {
        let options = Options::_from_params(_create_params(vec!["-w", "foobar"]));
        assert_eq!(options.window_id, "foobar");
    }

    #[test]
    fn when_argument_window_id_is_not_given_then_option_is_empty_string() {
        let options = Options::_from_params(_create_params(vec![]));
        assert_eq!(options.window_id, "");
    }

    fn _create_params(custom_params: Vec<&str>) -> Vec<&str> {
        [vec!["xgifwallpaper"], custom_params, vec![PATH_TO_GIF]].concat()
    }
}
