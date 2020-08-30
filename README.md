# xgifwallpaper

Use an animated GIF as wallpaper on X11-systems.

By using shared memory between X11 client and server, this is not as 
performance-inefficient as it may seem at first. Nonetheless expect some
memory to be used for bigger GIFs with a lot of frames.

Due to using the shared memory extenstion of X11, this program will not work
in X11 sessions over the network.

Some window managers may hide the X11 root window, like Gnome does. There, you
will see no visible effect.


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
    -d, --default-delay <default-delay>    Delay in centiseconds between frames, if unspecified in GIF. [default: 10]

ARGS:
    <PATH_TO_GIF>    Path to GIF-file
```

### Examples

`xgifwallpaper mybackground.gif`

`xgifwallpaper -b "#ffaa00" mybackground.gif`

`xgifwallpaper -d 10 mybackground.gif`


## Runtime dependencies

Dynamically links these X11-libs at runtime:

* xlib
* xinerama
* xshm

## Build

### Install Rust development environment

See [Installing Rust](https://www.rust-lang.org/learn/get-started).

### Install X11-Header-files

You need the header files for _X11_ itself and for the extenstions _Xinerama_
and _XShm_.

On *Arch*-based-systems, use

```console
# pacman -S libx11 libxinerama libxext
```

On *Ubuntu*-based-systems, use

```console
$ sudo apt install libx11-dev libxinerama-dev libxshm-dev
```

This should also work on Debian, but it is not verified.

### Actual build

From project-root:

```console
$ cargo build --release
```

The result will be built as `target/release/xgifwallpaper`.

