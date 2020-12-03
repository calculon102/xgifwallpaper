use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use xgifwallpaper::options::Options;
use xgifwallpaper::screens::Screens;
use xgifwallpaper::xcontext::XContext;
use xgifwallpaper::*;

/// Application entry-point
fn main() {
    let options = Arc::new(Options::from_args());
    let running = Arc::new(AtomicBool::new(true));

    init_sigint_handler(options.clone(), running.clone());

    let xcontext = match XContext::new(options.clone()) {
        Ok(xcontext) => Box::new(xcontext),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(e.code);
        }
    };

    let mut wallpapers = render_wallpapers(
        &xcontext,
        Screens::query_x_screens(),
        options.clone(),
        running.clone(),
    );

    clear_background(&xcontext, options.clone());

    do_animation(&xcontext, &mut wallpapers, options.clone(), running.clone());

    clean_up(xcontext, wallpapers, options);
}

/// Register handler for interrupt-signal.
fn init_sigint_handler<'a>(options: Arc<Options>, running: Arc<AtomicBool>) {
    let verbose = options.verbose;

    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);

        if verbose {
            println!("SIGINT received");
        }
    })
    .expect("Error setting Ctrl-C handler");
}
