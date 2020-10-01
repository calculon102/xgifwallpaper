// TODO Document

use crate::screen_info::*;

pub enum ImagePlacementStrategy {
    CENTER,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Position {
    CENTER,
    FILL,
    MAX,
}

#[derive(Debug)]
pub struct ImagePlacement {
    pub src_x: i32,
    pub src_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Hash, Debug)]
pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

impl Resolution {
    pub fn new(width: i32, height: i32) -> Resolution {
        Resolution { width, height }
    }
}

pub fn compute_target_resolution(
    image_resolution: &Resolution,
    screen_resolution: &Resolution,
    target_position: &Position,
) -> Resolution {
    match *target_position {
        Position::CENTER => image_resolution.clone(),
        Position::FILL => compute_fill_resolution(image_resolution, screen_resolution),
        Position::MAX => panic!("MAX not implemented!"),
    }
}

fn compute_fill_resolution(
    image_resolution: &Resolution,
    screen_resolution: &Resolution,
) -> Resolution {
    let mut result = Resolution::new(0, 0);

    let d_width = screen_resolution.width - image_resolution.width;
    let d_height = screen_resolution.height - image_resolution.height;

    if d_width == 0 && d_height == 0 {
        result.width = screen_resolution.width;
        result.height = screen_resolution.height;
    } else {
        if d_width >= d_height {
            result.width = screen_resolution.width;

            let scale = screen_resolution.width as f32 / image_resolution.width as f32;
            result.height = (image_resolution.height as f32 * scale) as i32;
        } else {
            result.height = screen_resolution.height;

            let scale = screen_resolution.height as f32 / image_resolution.height as f32;
            result.width = (image_resolution.width as f32 * scale) as i32;
        }
    }

    result
}

pub fn get_image_placement(
    image_resolution: &Resolution,
    screen: &Screen,
    strategy: ImagePlacementStrategy,
) -> ImagePlacement {
    match strategy {
        ImagePlacementStrategy::CENTER => {
            center_image(image_resolution.width, image_resolution.height, screen)
        }
    }
}

// TODO Test
fn center_image(width: i32, height: i32, screen: &Screen) -> ImagePlacement {
    let mut out = ImagePlacement {
        src_x: 0,
        src_y: 0,
        dest_x: screen.x_org,
        dest_y: screen.y_org,
        width,
        height,
    };

    if width > screen.width {
        out.src_x = (width - screen.width) / 2;
        out.width = screen.width;
    }

    if height > screen.height {
        out.src_y = (height - screen.height) / 2;
        out.height = screen.height;
    }

    if screen.width > width {
        out.dest_x = screen.x_org + ((screen.width - width) / 2);
    }

    if screen.height > height {
        out.dest_y = screen.y_org + ((screen.height - height) / 2);
    }

    return out;
}

#[test]
fn when_position_is_center_then_target_resolutions_equals_image_resolution() {
    use std::collections::HashSet;

    let mut screen_resolutions: HashSet<Resolution> = HashSet::new();
    screen_resolutions.insert(Resolution::new(1920, 1080));
    screen_resolutions.insert(Resolution::new(1080, 1920));
    screen_resolutions.insert(Resolution::new(2000, 2000));

    let image_resolution = Resolution::new(1000, 1000);

    let actual = compute_target_resolution(
        &image_resolution,
        &Resolution::new(1080, 1920),
        &Position::CENTER,
    );

    assert_eq!(true, actual == image_resolution);
}

#[test]
fn when_image_1000x1000_screen_1920_1080_then_target_1920_1920() {
    _test_compute_fill_resolution(
        Resolution::new(1000, 1000),
        Resolution::new(1920, 1080),
        Resolution::new(1920, 1920),
    );
}

#[test]
fn when_image_1000x1000_screen_1080_1920_then_target_1920_1920() {
    _test_compute_fill_resolution(
        Resolution::new(1000, 1000),
        Resolution::new(1080, 1920),
        Resolution::new(1920, 1920),
    );
}

#[test]
fn when_image_1920x1080_screen_1000_1000_then_target_1777_1000() {
    _test_compute_fill_resolution(
        Resolution::new(1920, 1080),
        Resolution::new(1000, 1000),
        Resolution::new(1777, 1000),
    );
}

#[test]
fn when_image_1080x1920_screen_1000_1000_then_target_1000_1777() {
    _test_compute_fill_resolution(
        Resolution::new(1080, 1920),
        Resolution::new(1000, 1000),
        Resolution::new(1000, 1777),
    );
}

#[test]
fn when_image_1920x1080_screen_1920_1080_then_target_1920_1080() {
    _test_compute_fill_resolution(
        Resolution::new(1920, 1080),
        Resolution::new(1920, 1080),
        Resolution::new(1920, 1080),
    );
}

#[test]
fn when_image_1000x1080_screen_1500_500_then_target_1500_1500() {
    _test_compute_fill_resolution(
        Resolution::new(1000, 1000),
        Resolution::new(1500, 500),
        Resolution::new(1500, 1500),
    );
}

#[test]
fn when_image_1000x1080_screen_500_1500_then_target_1500_1500() {
    _test_compute_fill_resolution(
        Resolution::new(1000, 1000),
        Resolution::new(500, 1500),
        Resolution::new(1500, 1500),
    );
}

fn _test_compute_fill_resolution(image: Resolution, screen: Resolution, expected: Resolution) {
    let actual = compute_fill_resolution(&image, &screen);

    if actual != expected {
        eprintln!("actual != expected: {:?} != {:?}", actual, expected);
    }

    assert_eq!(true, actual == expected);
}
