use std::{
    fs::File,
    io::{BufReader, Stdout, Write},
    num::NonZeroU32,
    path::PathBuf,
    process::exit,
    rc::Rc,
    str::FromStr,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
};

use args::BrowserArgs;
use clap::Parser;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{read, KeyCode, KeyEvent, KeyEventKind, KeyEventState},
    execute, queue,
    style::{Print, Stylize},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use neb_core::{
    document::parse_from_stream,
    gfx::vello::{
        kurbo::{Affine, Line, Point},
        peniko::{Brush, Color, Stroke},
    },
    ids::{get_id_mgr, ID},
};

use neb_util::format::TreeDisplay;

mod args;

pub struct State {
    debug_id: Option<ID>,
    debug_line: Option<NonZeroU32>,
}

fn main() {
    env_logger::init();

    let args = BrowserArgs::parse();

    let file = File::open(
        args.view
            .unwrap_or(PathBuf::from_str("test_files/messages.smf").unwrap()),
    )
    .unwrap();
    let file = BufReader::new(file);

    let document = Arc::new(parse_from_stream(file));

    let errors = document.get_errors();
    if errors.len() > 0 {
        for e in errors {
            println!("{}", e)
        }
        return;
    };

    let (tx, rx) = mpsc::channel();

    let io_doc = document.clone();

    std::panic::set_hook(Box::new(|info| {
        let mut stdout = std::io::stdout();
        stdout.flush().unwrap();
        execute!(stdout, Show, LeaveAlternateScreen).unwrap();

        crossterm::terminal::disable_raw_mode().unwrap();

        println!("{}", info);

        exit(0)
    }));

    if args.debug_inspector {
        std::thread::spawn(move || {
            let tx = Rc::new(tx);
            crossterm::terminal::enable_raw_mode().unwrap();

            let mut stdout = std::io::stdout();

            queue!(stdout, EnterAlternateScreen, Hide).unwrap();

            stdout.flush().unwrap();

            let i = Rc::new(RwLock::new(0));

            let print = |stdout: &mut Stdout,
                         value: Rc<RwLock<u32>>,
                         index: u32,
                         on_selection: Rc<Box<dyn Fn(u64)>>| {
                let st = io_doc
                    .get_body()
                    .borrow()
                    .format_unformat(Box::new(move |element, c| {
                        let res = {
                            let i = value.read().unwrap();
                            if *i == index {
                                (*on_selection)(element.get_user_data().unwrap());
                                Some(format!("{}", c.black().on_white()))
                            } else {
                                None
                            }
                        };

                        {
                            let mut i = value.write().unwrap();
                            *i += 1;
                        }

                        res
                    }));
                let lines = st.split("\n");
                for (y, line) in lines.enumerate() {
                    queue!(stdout, MoveTo(1, y as _), Print(line.to_string())).unwrap();
                }
            };

            let mut index = 0;

            // let src = max.clone();

            {
                let tx = tx.clone();
                let fui = i.clone();
                print(
                    &mut stdout,
                    fui,
                    index,
                    Rc::new(Box::new(move |value: u64| {
                        tx.send((value, 0)).unwrap();
                    })),
                );
                stdout.flush().unwrap();
            }
            // Rc

            let max = Rc::new(*i.read().unwrap());
            let src = Rc::clone(&max);
            let select: Rc<Box<dyn Fn(u64)>> = Rc::new(Box::new(move |value: u64| {
                tx.send((value, *src)).unwrap();
            }));

            loop {
                // `read()` blocks until an `Event` is available
                match read().unwrap() {
                    crossterm::event::Event::Key(KeyEvent {
                        code: KeyCode::Char('q') | KeyCode::Esc,
                        ..
                    }) => {
                        stdout.flush().unwrap();
                        execute!(stdout, Show, LeaveAlternateScreen).unwrap();

                        crossterm::terminal::disable_raw_mode().unwrap();

                        std::process::exit(0);
                    }

                    crossterm::event::Event::Key(KeyEvent {
                        code: KeyCode::Up,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        {
                            let mut i = i.write().unwrap();
                            *i = 0;
                        }
                        if index > 0 {
                            let fui = i.clone();
                            index -= 1;
                            print(&mut stdout, fui, index, select.clone());

                            stdout.flush().unwrap();
                        }
                    }
                    crossterm::event::Event::Key(KeyEvent {
                        code: KeyCode::Down,
                        kind: KeyEventKind::Press,
                        ..
                    }) => {
                        {
                            let mut i = i.write().unwrap();
                            *i = 0;
                        }
                        if index < *max - 1 {
                            let fui = i.clone();
                            index += 1;
                            print(&mut stdout, fui, index, select.clone());
                            stdout.flush().unwrap();
                        }
                    }
                    // crossterm::event::Event::Key(event) => println!("{:?}", event),
                    _ => (),
                }
            }
        });
    }

    let state = Arc::new(RwLock::new(State {
        debug_id: None,
        debug_line: None,
    }));

    pollster::block_on(neb_core::gfx::start_graphics_thread(move |builder| {
        document.layout(builder.size.width, builder.size.height);

        document.draw(builder);

        if args.debug_inspector {
            match rx.try_recv() {
                Ok(val) => {
                    let mut m = state.write().unwrap();
                    m.debug_id = Some(val.0);
                    m.debug_line = NonZeroU32::new(val.1)
                }
                _ => (),
            }
        }

        let m = state.read().unwrap();

        if let Some(val) = &m.debug_id {
            let idmgr = get_id_mgr();
            let layout = idmgr.get_layout(*val);

            builder.builder.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                &Brush::Solid(Color::RED),
                None,
                &layout.padding_rect,
                // Line::new(Point::new(layout.content_rect., y), p1),
            );

            builder.builder.stroke(
                &Stroke::new(2.0),
                Affine::IDENTITY,
                &Brush::Solid(Color::GREEN),
                None,
                &layout.content_rect,
                // Line::new(Point::new(layout.content_rect., y), p1),
            );
        }

        if let (Some(val), Some(line)) = (&m.debug_id, &m.debug_line) {
            let idmgr = get_id_mgr();
            let layout = idmgr.get_layout(*val);

            let mut stdout = std::io::stdout();
            execute!(
                stdout,
                MoveTo(1, 1 + line.get() as u16),
                Print(format!(
                    "Content {} Padding {}",
                    layout.content_rect, layout.padding_rect
                ))
            )
            .unwrap();
        }
    }))
    .unwrap();
}
