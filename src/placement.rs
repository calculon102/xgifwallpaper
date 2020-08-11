use crate::screen_info::*;

pub enum ImagePlacementStrategy {
    CENTER,
}

pub struct ImagePlacement {
    pub src_x: i32,
    pub src_y: i32,
    pub dest_x: i32,
    pub dest_y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn get_image_placement(
    width: i32,
    height: i32,
    screen: Screen,
    strategy: ImagePlacementStrategy,
) -> ImagePlacement {
    match strategy {
        ImagePlacementStrategy::CENTER => center_image(width, height, screen),
    }
}

fn center_image(width: i32, height: i32, screen: Screen) -> ImagePlacement {
    let mut out = ImagePlacement {
        src_x: 0,
        src_y: 0,
        dest_x: 0,
        dest_y: 0,
        width: width,
        height: height,
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
