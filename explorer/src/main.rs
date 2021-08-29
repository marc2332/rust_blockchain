use argh::FromArgs;
use client::RPCClient;
use crossterm::{
    event::{
        self,
        DisableMouseCapture,
        EnableMouseCapture,
        Event as CEvent,
        KeyCode,
    },
    execute,
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::{
    error::Error,
    io::{
        stdout,
        Stdout,
    },
    sync::{
        mpsc,
        Arc,
        Mutex,
    },
    thread,
    time::{
        Duration,
        Instant,
    },
};
use tui::{
    backend::CrosstermBackend,
    layout::{
        Alignment,
        Constraint,
        Direction,
        Layout,
    },
    style::{
        Color,
        Style,
    },
    widgets::{
        Block,
        Borders,
        Paragraph,
        Wrap,
    },
    Frame,
    Terminal,
};

use std::time;

enum Event<I> {
    Input(I),
    Tick,
}

/// Crossterm demo
#[derive(Debug, FromArgs)]
struct Cli {
    /// time in ms between two ticks.
    #[argh(option, default = "250")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    #[argh(option, default = "true")]
    enhanced_graphics: bool,
}

fn nodes_list(f: &mut Frame<CrosstermBackend<Stdout>>, data: Vec<usize>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(f.size());

    for i in 0..5 {
        let paragraph = Paragraph::new("█".repeat(data[i]))
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(format!(
                        "node {} (Block height: {})",
                        i,
                        data[i].to_string()
                    ))
                    .borders(Borders::ALL),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, chunks[i]);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli: Cli = argh::from_env();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(cli.tick_rate);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));
            if event::poll(timeout).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    terminal.clear()?;
    let data = Arc::new(Mutex::new(vec![0; 5]));
    for i in 0..5 {
        let data = data.clone();
        tokio::spawn(async move {
            loop {
                let client = RPCClient::new(&format!("http://localhost:{}", 2000 + i))
                    .await
                    .unwrap();

                data.lock().unwrap()[i] = client.get_chain_length().await.unwrap_or(0);

                let ten_millis = time::Duration::from_millis(1000);

                thread::sleep(ten_millis);
            }
        });
    }

    loop {
        terminal.draw(|f| {
            nodes_list(f, data.lock().unwrap().to_vec());
        })?;
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode()?;
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )?;
                    terminal.show_cursor()?;
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}
