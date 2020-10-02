# xgifwallpaper

Use an animated GIF as wallpaper on X11-systems.

`xgifwallpaper` draws on the root window. On window-managers, where the root
window is hidden, there will be no visible effect. Examples are KDE Plasma and
Gnome.

By using shared memory between X11 client and server, this is not as 
performance-inefficient as it may seem at first. Nonetheless, expect some
memory to be used for bigger GIFs with a lot of frames.

Due to using the shared memory extension of X11, this program will not work
in X11 sessions over the network.

`xgifwallpaper` will use all available screens.

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
    -s, --scale <SCALE>                    Scaling of frames, relative to available screen [default: NONE]
                                           values: NONE, FILL, MAX]

ARGS:
    <PATH_TO_GIF>    Path to GIF-file
```

### Examples

Center `mybackground.gif` on all screens:

`xgifwallpaper mybackground.gif`

Set background color to `#ffaa00`:

`xgifwallpaper -b "#ffaa00" mybackground.gif`

Override default delay with 100 centiseconds:

`xgifwallpaper -d 100 mybackground.gif`

Scale `mybackground.gif` to fill the entire screen and be verbose:

`xgifwallpaper -v -s FILL mybackground.gif`

Scale `mybackground.gif` to maximize used screen-space, but without cutting the
image. Also, set background-color to white, override default-delay with 30
centiseconds and be verbose:

`xgifwallpaper -v -b white -d 30 -s MAX mybackground.gif`

## Install

There is an [AUR-package](https://aur.archlinux.org/packages/xgifwallpaper/)
for arch-based linux-systems.

For other systems, you will need to build this yourself.

### Runtime dependencies

Dynamically links mainly X11-libs:

* `xlib`
* `xinerama`
* `xshm`

There will be build-specific dependencies

## Build

### Install Rust development environment

See [Installing Rust](https://www.rust-lang.org/learn/get-started).

### Install dependencies

You need the header files for `X11` and its extensions `Xinerama` and `XShm`.
Further dependencies are `libc` and a C-compiler-suite like `gcc`, rust will
need to link the C-bindings.

#### Arch

On *Arch*-based-systems, the packages needed are listed as `depends` and
`makedepends` in the
[PKGBUILD](https://aur.archlinux.org/cgit/aur.git/tree/PKGBUILD?h=xgifwallpaper)
of the AUR-package:

```console
# pacman -S gcc gcc-libs git glibc libx11 libxau libxcb libxdmcp libxext libxinerama
```

Rust is not included, as I would suggest installing it the way described above.

#### Ubuntu

On *Ubuntu*-based-systems, use

```console
$ sudo apt install libx11-dev libxinerama-dev libxshm-dev
```

`git`, `rust`, `libc` and a C-compiler-suite need to be installed.

This should also work on Debian, but this is not verified.

### Actual build

From project-root:

```console
$ cargo build --release
```

The result is built as `target/release/xgifwallpaper`.

