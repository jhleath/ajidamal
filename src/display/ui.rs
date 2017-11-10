extern crate chrono;

use std::io;
use std::thread::{self, JoinHandle};
use std::time::{Duration};

use super::{Screen};
use super::base::{Color};
use super::text::{TextRenderer};
use super::view::{Buffer, Delegate, View};

use self::chrono::prelude::*;

// 10ms sleeping time gets us to around 100hz - actual processing
// time.
const UI_THREAD_SLEEP_MS: u64 = 100;

pub struct Interface {
    thread_handler: JoinHandle<()>
}

impl Interface {
    pub fn new(fb_path: String) -> Interface {
        Interface {
            thread_handler: Self::start_thread(fb_path).unwrap(),
        }
    }

    pub fn exit(self) {
        // TODO: Shut down the interface
        println!("{:?}", self.thread_handler.join());
    }

    // The UI should run on a separate thread from the application
    // logic so that it stays somewhat responsive.
    pub fn start_thread(fb_path: String) -> io::Result<JoinHandle<()>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("aji/ui".to_string()).spawn(
            move || {
                let mut screen = Screen::new(fb_path);
                let mut root_view = Buffer::new(screen.width as u64, screen.height as u64);
                let text_renderer = TextRenderer::new();

                let mut status_bar = StatusBar::new(/*height=*/17);

                loop {
                    let mut changed = false;

                    if status_bar.needs_redraw() {
                        changed = true;
                        status_bar.draw(&mut View::new_full(&mut root_view), &text_renderer);
                    }

                    if changed {
                        screen.render_view(&root_view);
                    }

                    thread::sleep(Duration::from_millis(UI_THREAD_SLEEP_MS));
                }
            })
    }
}

#[derive(Debug)]
struct StatusBar {
    height: u64,
    render_time: Option<DateTime<Local>>,
}

impl Delegate for StatusBar {
    fn needs_redraw(&self) -> bool {
        match self.render_time {
            None => true,
            Some(t) => (Local::now().signed_duration_since(t) > chrono::Duration::minutes(1))
        }
    }

    fn draw(&mut self, view: &mut View, text: &TextRenderer) {
        let (mut i, mut j) = (0, 0);
        while j < self.height {
            while i < view.width() {
                view.write_pixel(i as usize, j as usize,
                                 Color::gray(/*intensity=*/255));

                i += 1;
            }

            i = 0;
            j += 1;
        }

        let local = Local::now();
        let time_buffer = text.rasterize(/*size=*/14.0, Color::gray(/*intensity=*/0),
                                         format!("{}", local.format("%H:%M %p")));

        // 5 pixels of padding on the right
        let text_start_x = (view.width() - time_buffer.width()) - 5;
        view.render_full(&time_buffer, text_start_x, /*y=*/1);


        self.render_time = Some(Local::now());
    }
}

impl StatusBar {
    fn new(height: u64) -> StatusBar {
        StatusBar {
            height: height,
            render_time: None
        }
    }
}
