
// Derived from the example at
// https://github.com/fdehau/tui-rs/blob/a6b25a487786534205d818a76acb3989658ae58c/examples/user_input.rs
// Thanks to the authors.
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;
use std::path::PathBuf;
use clap::Parser;

use std::fs::File;
use std::io::BufReader;

use itertools::Itertools;

mod app;
use crate::app::JsonAppState;

// TEST: invalid args
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// JSON file containing tree
    #[clap(parse(from_os_str))]
    input_file: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    // TODO: try to make sure not to panic here- this leaves the terminal in a bad state
    // TEST: panic
    let args = Args::parse();

    // Read input file
    let file = File::open(args.input_file)?;
    // TEST: no file
    let reader = BufReader::new(file);
    // TEST: invalid json
    // TEST: json that we can't/don't use, e.g. arrays (or write a serde deserialize trait)
    let v: serde_json::Value = serde_json::from_reader(reader)?;

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = JsonAppState::new(&v);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Err(err) => println!("{:?}", err),
        Ok(Some(s)) => println!("{}", s),
        Ok(None) => {}, // User exited
    }

    Ok(())
}

fn run_app<'a, B: Backend>(terminal: &mut Terminal<B>, mut app: JsonAppState) -> io::Result<Option<String>> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key {
                crokey::key!(ctrl-c) => {
                    return Ok(None);
                }
                crokey::key!(enter) => {
                    let res = app.push_selection();
                    if app::NodeType::Leaf == res {
                        return Ok(Some(app.get_history().to_string()))
                    }
                }
                crokey::key!(ctrl-u) => {
                    app.set_filter(String::new());
                }
                crokey::key!(ctrl-j) => {
                    app.select_next();
                }
                crokey::key!(ctrl-k) => {
                    app.select_prev();
                }
                crokey::key!(ctrl-w) => {
                    let mut filter = app.get_filter().to_owned();
                    if filter.len() == 0 {
                        app.pop_selection();
                    } else {
                        filter.truncate(filter.trim_end().rfind(' ').map(|n| n + 1).unwrap_or(0));
                        app.set_filter(filter);
                    }
                }
                KeyEvent { code: KeyCode::Char(c), .. } => {
                    let mut filter = app.get_filter().to_owned();
                    filter.push(c);
                    app.set_filter(filter);
                }
                crokey::key!(backspace) => {
                    let mut filter = app.get_filter().to_owned();
                    if filter.len() == 0 {
                        app.pop_selection();
                    } else {
                        filter.pop();
                        app.set_filter(filter);
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &JsonAppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = (
        vec![
            Span::raw("Press "),
            Span::styled("ctrl+c", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to exit")
        ],
        Style::default()
    );
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

    let filter = app.get_filter();
    let history = app.get_history();
    // TODO: colour this differently:
    // let input_text = format!("{} {}", history, filter);
    let input_text: String = history.iter()
        .map(|o| o.to_string())
        .chain(std::iter::once(filter.clone()))
        .intersperse(" ".to_string())
        .collect();
    let width = input_text.width();
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Filter"));
    f.render_widget(input, chunks[1]);
    // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
    f.set_cursor(
        // Put cursor past the end of the input text
        chunks[1].x + width as u16 + 1,
        // Move one line down, from the border to the input line
        chunks[1].y + 1,
    );

    let candidate_list: Vec<ListItem> = match app.choices() {
        Some((before, selected, after)) => {
            before.iter()
                .map(|e| ListItem::new(vec![Spans::from(Span::raw(format!("{}", e)))]))
                .chain(std::iter::once(ListItem::new(vec![Spans::from(Span::raw(format!("{}", selected)))]).style(Style::default().bg(Color::Blue))))
                .chain(
                    after.iter().map(|e| ListItem::new(vec![Spans::from(Span::raw(format!("{}", e)))]))
                )
                .collect()
        },
        None => Vec::new(),
    };
    let messages =
        List::new(candidate_list).block(Block::default().borders(Borders::ALL).title("Matches"));
    f.render_widget(messages, chunks[2]);
}
