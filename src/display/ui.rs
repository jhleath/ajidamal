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

                let status_bar_height = 17;
                let mut status_bar = StatusBar::new();

                // The main view has the ability to render 200
                // ContentViews of 50 pixels each in its internal
                // buffer.
                let mut main_view = MainView::new(screen.width as u64, screen.height as u64 - status_bar_height,
                                                  screen.width as u64, 1000);

                main_view.add_content(ContentView::new("John Smith".to_string(),
                                                       "Text Message 1".to_string(),
                                                       Local::now()));
                main_view.add_content(ContentView::new("Jane Doe".to_string(),
                                                       "Other Media 2".to_string(),
                                                       Local::now()));
                main_view.add_content(ContentView::new("Offscreen".to_string(),
                                                       "Other Media 3".to_string(),
                                                       Local::now()));

                loop {
                    let mut changed = false;

                    if status_bar.needs_redraw() {
                        changed = true;
                        status_bar.draw(
                            &mut View::new(Rect::from_origin(screen.width as u64, status_bar_height),
                                           &mut root_view),
                            &text_renderer);
                    }

                    if main_view.needs_redraw() {
                        changed = true;
                        main_view.draw(
                            &mut View::new(Rect::new(Point::new(/*x=*/0, status_bar_height),
                                                     screen.width as u64,
                                                     screen.height as u64 - status_bar_height),
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
    buffer: Buffer,
    content: Vec<ContentView>,
    bounds: Rect,
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
        let (width, height) = (view.width(), view.height());
        assert!(width == self.bounds.width);
        assert!(height == self.bounds.height);

        let mut i = 0;
        for c in self.content.iter_mut() {
            // TODO: [hleath 2017-11-12] Right now, all
            // ContentViews are 50 pixels high. They should have a
            // dynamic height depending on the amount of data to
            // display.
            let content_view_height = 50;
            let padding = 2;
            let y = ((content_view_height + padding) * i) + padding;
            if y + content_view_height >= self.buffer.height() {
                // Trim any content views that would run off the end
                // of the scroll buffer.
                break;
            }

            if c.needs_redraw() || !self.drawn {
                let mut content_view = View::new(
                    Rect::new(Point::new(/*x=*/padding, y),
                              width - (padding * 2), content_view_height),
                    &mut self.buffer
                );

                c.draw(&mut content_view, text);
            }
            i += 1;
        }

        view.render(&self.buffer, Rect::from_origin(width, height), self.bounds);
        self.drawn = true;

        // self.bounds = self.debug_calculate_new_fake_scroll(self.bounds);
        // self.mark_dirty()
    }
}

impl MainView {
    fn new(width: u64, height: u64, buffer_width: u64, buffer_height: u64) -> MainView {
        // The buffer must be strictly larger than the underlying
        // width/height.
        assert!(width <= buffer_width && height <= buffer_height);

        MainView {
            drawn: false,
            content: Vec::new(),
            bounds: Rect::from_origin(width, height),
            buffer: Buffer::new(buffer_width, buffer_height)
        }
    }

    // TODO: [hleath 2017-11-12] Remove this when we have user input
    // controlling the scrolling.
    fn _debug_calculate_new_fake_scroll(&self, bounds: Rect) -> Rect {
        // TODO: [hleath 2017-11-12] Implement fake scrolling behavior
        bounds
    }

    fn mark_dirty(&mut self) {
        self.drawn = false;
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
