# xgifwallpaper

Use an animated GIF as wallpaper on X11-systems.

By using shared memory between X11 client and server, this is not as 
performance-inefficient as it may seem at first. Nonetheless, expect some
memory to be used for bigger GIFs with a lot of frames.

Due to using the shared memory extenstion of X11, this program will not work
in X11 sessions over the network.

Some window managers may hide the X11 root window, like KDE Plasma and Gnome do.
In that case, there will be no visible effect.


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

This assumes, `git`, `rust`, `libc` and a C-compiler-suite are already
installed.

This should also work on Debian, but this is not verified.

### Actual build

From project-root:

```console
$ cargo build --release
```

The result will be built as `target/release/xgifwallpaper`.

