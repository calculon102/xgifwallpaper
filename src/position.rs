//! Compute position and resolution of images according to screen-resolutions
//! and options for placement and scaling.
use crate::screen_info::*;

/// Determines how an image is to be aligned, relative to a screen.
pub enum Alignment {
    CENTER,
}

/// Scaling-options. All options respect aspect-ratio.
#[derive(Debug, PartialEq, Eq)]
pub enum Scaling {
    /// Don't scale
    NONE,
    /// Image should fill the whole screen, even if cut off.
    FILL,
    /// Image should be as large as possible, without losing content.
    MAX,
}

/// Coordinates to place an image.
#[derive(Debug, Eq, PartialEq)]
pub struct ImagePlacement {
    /// x-origin of the image raster to use.
    pub src_x: i32,
    /// y-origin of the image raster to use.
    pub src_y: i32,
    /// x-origin of the screen to use.
    pub dest_x: i32,
    /// y-origin of the screen to use.
    pub dest_y: i32,
    /// width of the image to render, relative to src_x.
    pub width: u32,
    /// height of the image to render, relative to src_y.
    pub height: u32,
}

impl ImagePlacement {
    pub fn new(
        src_x: i32,
        src_y: i32,
        dest_x: i32,
        dest_y: i32,
        width: u32,
        height: u32,
    ) -> ImagePlacement {
        ImagePlacement {
            src_x,
            src_y,
            dest_x,
            dest_y,
            width,
            height,
        }
    }
}

/// Width and height as one unit.
#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Hash, Debug)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    /// Creates a new instance of `Resolution`.
    pub fn new(width: u32, height: u32) -> Resolution {
        Resolution { width, height }
    }

    /// Calculates the resolution an image should have to respect given scaling.
    pub fn fit_to_screen(&self, screen_resolution: &Resolution, scaling: &Scaling) -> Resolution {
        match *scaling {
            Scaling::NONE => self.clone(),
            Scaling::FILL => Resolution::scale_image_to_screen(self, screen_resolution, true),
            Scaling::MAX => Resolution::scale_image_to_screen(self, screen_resolution, false),
        }
    }

    /// Calculates the resolution an image should have to respect given scaling.
    ///
    /// If `fill` is `true` the whole screen is used, at cost of image-information.
    ///
    /// If `fill` is `false` the image is scaled as large as possible, without
    /// losing information.
    fn scale_image_to_screen(
        image_resolution: &Resolution,
        screen_resolution: &Resolution,
        fill: bool,
    ) -> Resolution {
        let mut result = Resolution::new(0, 0);

        let same_resolution = screen_resolution.width == image_resolution.width
            && screen_resolution.height == image_resolution.height;

        if same_resolution {
            result.width = screen_resolution.width;
            result.height = screen_resolution.height;
        } else {
            let screen_ratio = screen_resolution.width as f32 / screen_resolution.height as f32;
            let image_ratio = image_resolution.width as f32 / image_resolution.height as f32;

            let scale_to_width = if fill {
                screen_ratio > image_ratio
            } else {
                screen_ratio < image_ratio
            };

            if scale_to_width {
                result.width = screen_resolution.width;

                let scale = screen_resolution.width as f32 / image_resolution.width as f32;
                result.height = (image_resolution.height as f32 * scale) as u32;
            } else {
                result.height = screen_resolution.height;

                let scale = screen_resolution.height as f32 / image_resolution.height as f32;
                result.width = (image_resolution.width as f32 * scale) as u32;
            }
        }

        result
    }

    /// Computes coordinates of image for given alignment on a screen.
    pub fn position_on_screen(&self, screen: &Screen, alignment: Alignment) -> ImagePlacement {
        match alignment {
            Alignment::CENTER => Resolution::center_on_screen(self.width, self.height, screen),
        }
    }

    /// Places an image on screen, aligned to the center. Both horizontally and
    /// vertically.
    fn center_on_screen(width: u32, height: u32, screen: &Screen) -> ImagePlacement {
        let mut out = ImagePlacement::new(0, 0, screen.x_org, screen.y_org, width, height);

        if width > screen.width {
            out.src_x = ((width - screen.width) / 2) as i32;
            out.width = screen.width;
        }

        if height > screen.height {
            out.src_y = ((height - screen.height) / 2) as i32;
            out.height = screen.height;
        }

        if screen.width > width {
            out.dest_x = screen.x_org + ((screen.width - width) / 2) as i32;
        }

        if screen.height > height {
            out.dest_y = screen.y_org + ((screen.height - height) / 2) as i32;
        }

        return out;
    }
}

#[cfg(test)]
mod tests {
    use super::Alignment;
    use super::ImagePlacement;
    use super::Resolution;
    use super::Scaling;
    use super::Screen;

    #[test]
    fn when_position_is_center_then_target_resolutions_equals_image_resolution() {
        use std::collections::HashSet;

        let mut screen_resolutions: HashSet<Resolution> = HashSet::new();
        screen_resolutions.insert(Resolution::new(1920, 1080));
        screen_resolutions.insert(Resolution::new(1080, 1920));
        screen_resolutions.insert(Resolution::new(2000, 2000));

        let image_resolution = Resolution::new(1000, 1000);

        let actual = image_resolution.fit_to_screen(&Resolution::new(1080, 1920), &Scaling::NONE);

        assert_eq!(true, actual == image_resolution);
    }

    #[test]
    fn when_image_1000x1000_screen_1920_1080_position_fill_then_target_1920_1920() {
        _test_compute_fill_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1920, 1080),
            Resolution::new(1920, 1920),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_1080_1920_position_fill_then_target_1920_1920() {
        _test_compute_fill_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1080, 1920),
            Resolution::new(1920, 1920),
        );
    }

    #[test]
    fn when_image_1920x1080_screen_1000_1000_position_fill_then_target_1777_1000() {
        _test_compute_fill_resolution(
            Resolution::new(1920, 1080),
            Resolution::new(1000, 1000),
            Resolution::new(1777, 1000),
        );
    }

    #[test]
    fn when_image_1080x1920_screen_1000_1000_position_fill_then_target_1000_1777() {
        _test_compute_fill_resolution(
            Resolution::new(1080, 1920),
            Resolution::new(1000, 1000),
            Resolution::new(1000, 1777),
        );
    }

    #[test]
    fn when_image_1920x1080_screen_1920_1080_position_fill_then_target_1920_1080() {
        _test_compute_fill_resolution(
            Resolution::new(1920, 1080),
            Resolution::new(1920, 1080),
            Resolution::new(1920, 1080),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_1500_500_position_fill_then_target_1500_1500() {
        _test_compute_fill_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1500, 500),
            Resolution::new(1500, 1500),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_500_1500_position_fill_then_target_1500_1500() {
        _test_compute_fill_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(500, 1500),
            Resolution::new(1500, 1500),
        );
    }

    #[test]
    fn when_image_2x1_screen_2560_1440_position_fill_then_target_2560_2560() {
        _test_compute_fill_resolution(
            Resolution::new(2, 1),
            Resolution::new(2560, 1440),
            Resolution::new(2880, 1440),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_1920_1080_position_max_then_target_1080_1080() {
        _test_compute_max_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1920, 1080),
            Resolution::new(1080, 1080),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_1080_1920_position_max_then_target_1080_1080() {
        _test_compute_max_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1080, 1920),
            Resolution::new(1080, 1080),
        );
    }

    #[test]
    fn when_image_1920x1080_screen_1000_1000_position_max_then_target_1000_562() {
        _test_compute_max_resolution(
            Resolution::new(1920, 1080),
            Resolution::new(1000, 1000),
            Resolution::new(1000, 562),
        );
    }

    #[test]
    fn when_image_1080x1920_screen_1000_1000_position_max_then_target_562_1000() {
        _test_compute_max_resolution(
            Resolution::new(1080, 1920),
            Resolution::new(1000, 1000),
            Resolution::new(562, 1000),
        );
    }

    #[test]
    fn when_image_1920x1080_screen_1920_1080_position_max_then_target_1920_1080() {
        _test_compute_max_resolution(
            Resolution::new(1920, 1080),
            Resolution::new(1920, 1080),
            Resolution::new(1920, 1080),
        );
    }

    #[test]
    fn when_image_1000x1000_screen_1500_500_position_max_then_target_500_500() {
        _test_compute_max_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(1500, 500),
            Resolution::new(500, 500),
        );
    }

    #[test]
    fn when_image_1000x1080_screen_500_1500_position_max_then_target_500_500() {
        _test_compute_max_resolution(
            Resolution::new(1000, 1000),
            Resolution::new(500, 1500),
            Resolution::new(500, 500),
        );
    }

    #[test]
    fn when_image_2x1_screen_2560_1440_position_max_then_target_2560_2560() {
        _test_compute_max_resolution(
            Resolution::new(2, 1),
            Resolution::new(2560, 1440),
            Resolution::new(2560, 1280),
        );
    }

    #[test]
    fn when_image_1x1_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(1, 1).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(0, 0, 1, 1, 1, 1));
    }

    #[test]
    fn when_image_1x3_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(1, 3).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(0, 0, 1, 0, 1, 3));
    }

    #[test]
    fn when_image_3x1_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(3, 1).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(0, 0, 0, 1, 3, 1));
    }

    #[test]
    fn when_image_3x3_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(3, 3).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(0, 0, 0, 0, 3, 3));
    }

    #[test]
    fn when_image_1x5_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(1, 5).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(0, 1, 1, 0, 1, 3));
    }

    #[test]
    fn when_image_5x1_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(5, 1).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(1, 0, 0, 1, 3, 1));
    }

    #[test]
    fn when_image_5x5_screen_3x3_then_center() {
        let screen = _create_screen0_3x3();
        let actual = Resolution::new(5, 5).position_on_screen(&screen, Alignment::CENTER);
        assert_eq!(actual, ImagePlacement::new(1, 1, 0, 0, 3, 3));
    }

    fn _test_compute_fill_resolution(image: Resolution, screen: Resolution, expected: Resolution) {
        _test_compute_resolution(image, screen, Scaling::FILL, expected);
    }

    fn _test_compute_max_resolution(image: Resolution, screen: Resolution, expected: Resolution) {
        _test_compute_resolution(image, screen, Scaling::MAX, expected);
    }

    fn _test_compute_resolution(
        image: Resolution,
        screen: Resolution,
        scaling: Scaling,
        expected: Resolution,
    ) {
        let actual = image.fit_to_screen(&screen, &scaling);

        if actual != expected {
            eprintln!("actual != expected: {:?} != {:?}", actual, expected);
        }

        assert_eq!(true, actual == expected);
    }

    fn _create_screen0_3x3() -> Screen {
        Screen {
            screen_number: 0,
            x_org: 0,
            y_org: 0,
            width: 3,
            height: 3,
        }
    }
}
