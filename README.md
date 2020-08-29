# xgifwallpaper

Use an animated GIF as wallpaper on X11-systems.

## Usage

See output of `--help`:

```
USAGE:
    xgifwallpaper [FLAGS] [OPTIONS] <PATH_TO_GIF>

FLAGS:
    -v               Verbose mode
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --background-color <X11-color>     X11 compilant color-name to paint background. [default: #000000]
    -d, --default-delay <default-delay>    Delay in centiseconds between frames, if the GIF does not specify itself.
                                           [default: 10]

ARGS:
    <PATH_TO_GIF>    Path to GIF-file
```

### Examples

`xgifwallpaper mybackground.gif`

`xgifwallpaper -b "#ffaa00" mybackground.gif`

`xgifwallpaper -d 10 mybackground.gif`


## Dependencies

Dynamically links these X11-libs at runtime:

* xlib
* xinerama
* xshm

To build the resective C-header files needed. On Arch-based systems these
may be aquired by

`# pacman -S libx11 libxinerama libxext`

## Installation

`cargo build --release`
