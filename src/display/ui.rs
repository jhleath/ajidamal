extern crate chrono;

use std::io;
use std::thread::{self, JoinHandle};
use std::time::{Duration};

use super::{Screen};
use super::base::{Color, Point, Rect};
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

                let mut status_bar = StatusBar::new();
                let mut main_view = MainView::new();

                loop {
                    let mut changed = false;

                    if status_bar.needs_redraw() {
                        changed = true;
                        status_bar.draw(
                            &mut View::new(Rect::from_origin(screen.width as u64, /*height=*/17),
                                           &mut root_view),
                            &text_renderer);
                    }

                    if main_view.needs_redraw() {
                        changed = true;
                        main_view.draw(
                            &mut View::new(Rect::new(Point::new(/*x=*/0, /*y=*/17),
                                                     screen.width as u64, screen.height as u64 - 17),
                                           &mut root_view),
                            &text_renderer);
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
struct ContentView {
    from: String,
    data: String,
    time: DateTime<Local>,
    drawn: bool
}

impl Delegate for ContentView {
    fn needs_redraw(&self) -> bool {
        !self.drawn
    }

    fn draw(&mut self, view: &mut View, text: &TextRenderer) {
        let box_color = Color::new(17, 132, 255);
        let (width, height) = (view.width(), view.height());

        view.draw_box(Point::origin(),
                      width as usize,
                      height as usize,
                      box_color);

        let from_buffer = text.rasterize(/*size=*/12.0, Color::gray(/*intensity=*/255),
                                         &self.from);
        let time_buffer = text.rasterize(/*size=*/10.0, Color::gray(/*intensity=*/255),
                                         &format!("{}", self.time.format("%H:%M %p")));
        let content_buffer = text.rasterize(/*size=*/14.0, Color::gray(/*intensity=*/255),
                                            &self.data);

        // 5 pixels of padding on the right
        let text_start_x = (view.width() - time_buffer.width()) - 1;
        view.render_full(&time_buffer, text_start_x, /*y=*/1);

        view.render_full(&from_buffer, /*x=*/2, /*y=*/1);

        view.render_full(&content_buffer, /*x=*/2, from_buffer.height());

        self.drawn = true;
    }
}

impl ContentView {
    fn new(from: String, data: String, time: DateTime<Local>) -> ContentView {
        ContentView {
            from: from,
            data: data,
            time: time,
            drawn: false
        }
    }
}

#[derive(Debug)]
struct MainView {
    drawn: bool,
    content: Vec<ContentView>,
}

impl Delegate for MainView {
    fn needs_redraw(&self) -> bool {
        let mut needs_redraw = !self.drawn;

        for c in self.content.iter() {
            needs_redraw = needs_redraw || c.needs_redraw();
        }

        needs_redraw
    }

    fn draw(&mut self, view: &mut View, text: &TextRenderer) {
        let (width, _height) = (view.width(), view.height());

        let mut i = 0;
        for c in self.content.iter_mut() {
            if c.needs_redraw() {
                let mut content_view = view.new_subview(
                    Rect::new(Point::new(/*x=*/2, /*y=*/(52 * i) + 2),
                              (width - 4), /*height=*/50));

                c.draw(&mut content_view, text);
            }
            i += 1;
        }

        self.drawn = true;
    }
}

impl MainView {
    fn new() -> MainView {
        MainView {
            drawn: false,
            content: Vec::new(),
        }
    }

    fn add_content(&mut self, c: ContentView) {
        self.content.push(c);
    }
}


#[derive(Debug)]
struct StatusBar {
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
        while j < view.height() {
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
                                         &format!("{}", local.format("%H:%M %p")));

        // 5 pixels of padding on the right
        let text_start_x = (view.width() - time_buffer.width()) - 5;
        view.render_full(&time_buffer, text_start_x, /*y=*/1);


        self.render_time = Some(Local::now());
    }
}

impl StatusBar {
    fn new() -> StatusBar {
        StatusBar {
            render_time: None
        }
    }
}
