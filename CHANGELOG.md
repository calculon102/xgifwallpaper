# Changelog

## [Unreleased](https://github.com/calculon102/xgifwallpaper/tree/HEAD)

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.3.2...master)

Nothing until now.


## [v0.3.2](https://github.com/calculon102/xgifwallpaper/tree/v0.3.2) - 2023-02-12

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.3.1...v0.3.2)

### Fixed

- warning: getting the inner pointer of a temporary CString [\#5](https://github.com/calculon102/xgifwallpaper/issues/5)
- fix usage of platform dependend `u8`, instead of `c_char` as parameter [\#10](https://github.com/calculon102/xgifwallpaper/issues/10)


## [v0.3.1](https://github.com/calculon102/xgifwallpaper/tree/v0.3.1) - 2020-11-30

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.3.0...v0.3.1)

### Added

- Compile feature `x11-integration-tests` to run tests against a running
X11-server.
- Experimental option `--scale-filter`. Default value `AUTO` does as as before
but value `PIXEL` uses most simple algorithm for performance and mabye better
suited for pixel-art.

### Fixed

- Crash when use --scale option [\#3](https://github.com/calculon102/xgifwallpaper/issues/3)
- Upscaling of pixel-art GIFs renders glitches [\#4](https://github.com/calculon102/xgifwallpaper/issues/4)


## [v0.3.0](https://github.com/calculon102/xgifwallpaper/tree/v0.3.0) - 2020-11-24

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.2.0...v0.3.0)

### Added

- Option `-w` to specify custom window to draw wallpaper on, instead of
X11-root. Useful for some window managers, which create custom windows as
background. Must be the same resolution as the screen though! This option
may also reference an atom of the root window by name, which contains a window
id.

### Fixed

- Query existing pixmap-properties and kill the owning application. He he he...


## [v0.2.0](https://github.com/calculon102/xgifwallpaper/tree/v0.2.0) - 2020-10-04

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.1.2...v0.2.0)

### Added

- This changelog
- Option `-s` to scale GIF to `FILL` screen or `MAX`-out as much as possible
- Sample GIFs and run-script as starter for semi-automated integration-tests

### Fixed

- Set background of root window to black on exit
- Exit gracefully, if there is no X display to open
- Exit gracefully, if given file is not a valid GIF


## [v0.1.2](https://github.com/calculon102/xgifwallpaper/tree/v0.1.2) - 2020-09-04

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.1.1...v0.1.2)

### Fixed

- Compositors get segmentation fault after closing program [\#2](https://github.com/calculon102/xgifwallpaper/issues/2)


## [v0.1.1](https://github.com/calculon102/xgifwallpaper/tree/v0.1.1) - 2020-09-03

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/v0.1.0...v0.1.1)

### Fixed

- Colors are not respected [\#1](https://github.com/calculon102/xgifwallpaper/issues/1)

## [v0.1.0](https://github.com/calculon102/xgifwallpaper/tree/v0.1.0) - 2020-08-29

[Full Changelog](https://github.com/calculon102/xgifwallpaper/compare/3b85a0131b52672b3f5c82d7d721b9a7c4da9769...v0.1.0)

### Added

- Animate GIF as background on root-window of a X-session
- Use `-b` to customize background-color for transparent or non-image pixels
- Use `-d` to specifiy a default-delay between frames, if none specified
- Use `-v` to be verbose about it
