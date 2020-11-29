[![aur.archlinux.org](https://img.shields.io/aur/version/xgifwallpaper)](https://aur.archlinux.org/packages/xgifwallpaper)

# `xgifwallpaper`

Use an animated GIF as wallpaper on X11-systems.

By using shared memory between X11 client and server, this is not as 
performance-inefficient as it may seem at first. Nonetheless, expect some
memory to be used for bigger GIFs with a lot of frames.

Due to using the shared memory extension of X11, this program will not work
in X11 sessions over the network.

In its current state, `xgifwallpaper` will always use all available screens.

## Compatibility

`xgifwallpaper` will work with all window managers, that expose the root
window of X. Also, X compositors supporting
[pseudo-transparency](https://en.wikipedia.org/wiki/Pseudo-transparency#XROOTPMAP_ID_and_ESETROOT_PMAP_ID_properties),
should work with `xgifwallpaper`, like `xcompmgr` or `picom`.

Currently known to work with

* [bspwm](https://github.com/baskerville/bspwm)
* [Cinnamon](https://github.com/linuxmint/Cinnamon)
* [dwm](https://dwm.suckless.org)
* [i3](https://i3wm.org)
* [Lxde](http://www.lxde.org) - Use -w option with the first windows ID from 
atom `_NET_CLIENT_LIST_STACKING(WINDOW)` of root window. See examples.
* [Mate](https://mate-desktop.org) - Use -w with `CAJA_DESKTOP_WINDOW_ID`
* [Openbox](https://github.com/danakj/openbox)
* [qtile](http://www.qtile.org/)
* [Xfce](https://www.xfce.org) - Use -w with `XFCE_DESKTOP_WINDOW`
* [xmonad](https://xmonad.org)

Known not work with 

* [Budgie](https://github.com/solus-project/budgie-desktop)
* [Englightenment](https://www.enlightenment.org)
* [Gnome3](https://www.gnome.org/gnome-3) /
[Mutter](https://gitlab.gnome.org/GNOME/mutter)
* [KDE Plasma 5](https://kde.org/plasma-desktop)

Every feedback and testing is appreciated!

## Usage

See output of `--help`:

```console
USAGE:
    xgifwallpaper [FLAGS] [OPTIONS] <PATH_TO_GIF>

FLAGS:
    -v               Verbose mode
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --background-color <X11-color>     X11 compilant color-name to paint background. [default: #000000]
    -d, --default-delay <default-delay>    Delay in centiseconds between frames, if unspecified in GIF. [default: 10]
    -s, --scale <SCALE>                    Scale GIF-frames, relative to available screen. [default: NONE]  [possible
                                           values: NONE, FILL, MAX]
    -w, --window-id <WINDOW_ID>            ID of window to animate wallpaper on its background, insted of the root
                                           window. As decimal, hex or name of root-atom.

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

Use window, referenced by specified atom of root-window, to draw wallpaper,
instead of the root window itself:

```bash
# Use with Mate
xgifwallpaper -w 'CAJA_DESKTOP_WINDOW_ID' my_wallpaper.gif

# Use with XFCE
xgifwallpaper -w 'XFCE_DESKTOP_WINDOW' my_wallpaper.gif
```

Use background the first window in stacking order to draw wallpaper, instead of
the root window. To be used with `Lxde`:

```bash   
xgifwallpaper -w $(xprop -root | awk '/_NET_CLIENT_LIST_STACKING\(WINDOW\)/{print $5}' | tr -d ,) mybackground.gif
```

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
$ sudo apt install libx11-dev libxinerama-dev libxext-dev
```

`git`, `rust`, `libc` and a C-compiler-suite need to be installed.

This should also work on Debian, but this is not verified.

### Actual build

From project-root:

```console
$ cargo build --release
```

The result is built as `target/release/xgifwallpaper`.

### Testing

Run `cargo test` for all unit-tests.

Run `cargo test --features x11-integration-tests` to run additional tests in a
X11-session, against the running server. 

Run `run_samples.sh` from directory `tests/samples` for an e2e-smoke-test of
some configurations.

