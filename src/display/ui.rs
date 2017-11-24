extern crate chrono;

use std::cmp;
use std::io;
use std::thread::{self, JoinHandle};
use std::time::{Duration};
use std::sync::mpsc;

use super::{Screen};
use super::base::{Color, Point, Rect};
use super::text::{TextRenderer};
use super::view::{Buffer, Delegate, View};

use super::frame_buffer::{FrameBuffer};

use self::chrono::prelude::*;

use gsm::sms::Message;

const UI_THREAD_SLEEP_MS: u64 = 50;

pub enum Command {
    SetMessages(Vec<Message>)
}

pub enum ScreenFactory {
    FrameBuffer(String),
    Simulator
}

pub struct Interface {
    pub sender: mpsc::Sender<Command>,
    thread_handler: JoinHandle<()>,
}

impl Interface {
    pub fn new(fb_path: String) -> Interface {
        Self::new_factory(ScreenFactory::FrameBuffer(fb_path))
    }

    pub fn new_factory(factory: ScreenFactory) -> Interface {
        let (send, recv) = mpsc::channel::<Command>();

        Interface {
            thread_handler: Self::start_thread(factory, recv).unwrap(),
            sender: send,
        }
    }

    pub fn exit(self) {
        // TODO: Shut down the interface
        println!("{:?}", self.thread_handler.join());
    }

    // The UI should run on a separate thread from the application
    // logic so that it stays somewhat responsive.
    pub fn start_thread(factory: ScreenFactory, receiver: mpsc::Receiver<Command>) -> io::Result<JoinHandle<()>> {
        // Create a reader thread to catch all responses from the
        // serial port
        thread::Builder::new().name("aji/ui".to_string()).spawn(
            move || {
                let mut screen: Box<Screen> = match factory {
                    ScreenFactory::FrameBuffer(path) => Box::new(FrameBuffer::new(path)),
                    ScreenFactory::Simulator => super::create_simulator()
                };

                let (width, height) = screen.dimensions();

                let mut root_view = Buffer::new(width as u64, height as u64);
                let text_renderer = TextRenderer::new();

                let status_bar_height = 17;
                let mut status_bar = StatusBar::new();

                // The main view has the ability to render 200
                // ContentViews of 50 pixels each in its internal
                // buffer.
                let mut main_view = MainView::new(width as u64, height as u64 - status_bar_height,
                                                  width as u64, 1000);

                main_view.add_content(ContentView::new("John Smith".to_string(),
                                                       "Text Message 1".to_string(),
                                                       Local::now()));
                main_view.add_content(ContentView::new("Jane Doe".to_string(),
                                                       "Other Media 2".to_string(),
                                                       Local::now()));
                main_view.add_content(ContentView::new("Offscreen".to_string(),
                                                       "Other Media 3".to_string(),
                                                       Local::now()));
                main_view.add_content(ContentView::new("More Offscreen".to_string(),
                                                       "Other Media 4".to_string(),
                                                       Local::now()));

                loop {
                    match receiver.try_recv() {
                        Ok(cmd) => {
                            match cmd {
                                Command::SetMessages(m) => {
                                    main_view.clear_content();
                                    for msg in m.into_iter() {
                                        main_view.add_content(ContentView::new(msg.sender,
                                                                               msg.contents,
                                                                               msg.time_stamp.with_timezone(&Local)));
                                    }
                                }
                            }
                        },
                        Err(mpsc::TryRecvError::Empty) => (),
                        Err(mpsc::TryRecvError::Disconnected) => {
                            return
                        }
                    }

                    let mut changed = false;

                    if status_bar.needs_redraw() {
                        changed = true;
                        status_bar.draw(
                            &mut View::new(Rect::from_origin(width as u64, status_bar_height),
                                           &mut root_view),
                            &text_renderer);
                    }

                    if main_view.needs_redraw() {
                        changed = true;
                        main_view.draw(
                            &mut View::new(Rect::new(Point::new(/*x=*/0, status_bar_height),
                                                     width as u64,
                                                     height as u64 - status_bar_height),
                                           &mut root_view),
                            &text_renderer);
                    }

                    if changed {
                        super::render_view(screen.as_mut(), &root_view);
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

        // TODO: [hleath 2017-11-13] This code just truncates the
        // content to the view. We should either... truncate the
        // string itself if it is too long, perform wrapping, or both.
        let min_width = cmp::min(width - 4, content_buffer.width());
        let min_height = cmp::min(height - from_buffer.height(), content_buffer.height());
        view.render(&content_buffer,
                    Rect::new(Point::new(2, from_buffer.height()),
                              min_width, min_height),
                    Rect::from_origin(min_width, min_height));

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
    used_height: Option<u64>,
    scrolling_down: bool
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

        // Draw the background
        view.draw_box(Point::origin(), width as usize, height as usize, Color::gray(/*intensity=*/0));

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

            if c.needs_redraw() {
                let mut content_view = View::new(
                    Rect::new(Point::new(/*x=*/padding, y),
                              width - (padding * 2), content_view_height),
                    &mut self.buffer
                );

                c.draw(&mut content_view, text);
            }

            // Keep the padding pixels at the bottom of the view as well.
            self.used_height = Some(y+ content_view_height + padding);
            i += 1;
        }

        view.render(&self.buffer, Rect::from_origin(width, height), self.bounds);
        self.drawn = true;
        self._debug_calculate_new_fake_scroll();
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
            buffer: Buffer::new(buffer_width, buffer_height),
            used_height: None,
            scrolling_down: true
        }
    }

    // TODO: [hleath 2017-11-12] Remove this when we have user input
    // controlling the scrolling.
    fn _debug_calculate_new_fake_scroll(&mut self) {
        match self.used_height {
            None => (),
            Some(h) => {
                let old_bounds = self.bounds;

                let new_bounds = if self.scrolling_down {
                    Rect::new(Point::new(/*x=*/0, old_bounds.origin.y + 1),
                              old_bounds.width, old_bounds.height)
                } else {
                    Rect::new(Point::new(/*x=*/0, old_bounds.origin.y - 1),
                              old_bounds.width, old_bounds.height)
                };

                if new_bounds.origin.y == 0 {
                    assert!(!self.scrolling_down);
                    self.scrolling_down = true;
                } else if new_bounds.origin.y + new_bounds.height == h {
                    assert!(self.scrolling_down);
                    self.scrolling_down = false;
                }

                self.bounds = new_bounds;
                self.mark_dirty()
            }
        }
    }

    fn mark_dirty(&mut self) {
        self.drawn = false;
    }

    fn add_content(&mut self, c: ContentView) {
        self.content.push(c);
        self.mark_dirty();
    }

    fn clear_content(&mut self) {
        self.content = Vec::new();
        self.mark_dirty();
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
