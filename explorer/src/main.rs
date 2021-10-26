use argh::FromArgs;
use client::NodeClient;
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
    convert::TryFrom,
    error::Error,
    io::stdout,
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
        Modifier,
        Style,
    },
    text::{
        Span,
        Spans,
    },
    widgets::{
        Block,
        Borders,
        Paragraph,
        Tabs,
        Wrap,
    },
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
}

struct TabsState {
    current: usize,
    titles: Vec<String>,
}

impl TabsState {
    pub fn new(titles: Vec<String>) -> Self {
        Self { current: 0, titles }
    }

    pub fn next(&mut self) {
        if self.current < self.titles.len() - 1 {
            self.current += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli: Cli = argh::from_env();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend).unwrap();
    terminal.clear().unwrap();

    let tabs_state = Arc::new(Mutex::new(TabsState::new(vec![
        "Blockchains".to_string(),
        "Addresses".to_string(),
    ])));

    // Setup input handling
    let (tx, rx) = mpsc::channel();

    let tick_rate = Duration::from_millis(cli.tick_rate);
    let tabs = tabs_state.clone();

    tokio::spawn(async move {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            if let Ok(res) = event::poll(timeout) {
                if res {
                    if let CEvent::Key(key) = event::read().unwrap() {
                        tx.send(Event::Input(key)).unwrap();
                    }
                }
                if last_tick.elapsed() >= tick_rate {
                    tx.send(Event::Tick).unwrap();
                    last_tick = Instant::now();
                }
            }
        }
    });

    let mut start_time = Instant::now();
    let times = Arc::new(Mutex::new(vec![1; 5]));
    let blockchain_data = Arc::new(Mutex::new(vec![("".to_string(), 0); 5]));
    let addresses_data = Arc::new(Mutex::new(vec![("-".to_string(), 0); 5]));
    let block_size = Arc::new(Mutex::new(50_u128));

    for i in 0..5 {
        let blockchain_data = blockchain_data.clone();
        let addresses_data = addresses_data.clone();
        let times = times.clone();
        let block_size = block_size.clone();

        tokio::spawn(async move {
            loop {
                let client = NodeClient::new(&format!("http://localhost:{}", 5000 + i))
                    .await
                    .unwrap();

                let blockchain_res = client
                    .get_chain_length()
                    .await
                    .unwrap_or(("".to_string(), 0));

                if blockchain_data.lock().unwrap()[i] != blockchain_res {
                    blockchain_data.lock().unwrap()[i] = blockchain_res.clone();
                    let duration = start_time.elapsed();
                    times.lock().unwrap()[i] = duration.as_millis();
                    start_time = Instant::now();
                }

                let node_address = client
                    .get_node_address()
                    .await
                    .unwrap_or_else(|_| "?".to_string());

                let address_data = client
                    .get_address_ammount(node_address.clone())
                    .await
                    .unwrap_or(0);

                if addresses_data.lock().unwrap()[i].1 != address_data {
                    addresses_data.lock().unwrap()[i] = (node_address, address_data);
                }

                if i == 0 {
                    let new_block = client.get_block_with_hash(blockchain_res.0).await;

                    if let Ok(Some(new_block)) = new_block {
                        *block_size.lock().unwrap() =
                            u128::try_from(new_block.transactions.len()).unwrap()
                    }
                }

                let ten_millis = time::Duration::from_millis(1000);
                thread::sleep(ten_millis);
            }
        });
    }

    loop {
        let blockchain_data = blockchain_data.clone();
        let addresses_data = addresses_data.clone();
        let times = times.clone();
        let tabs_state = tabs_state.clone();
        let block_size = block_size.clone();

        terminal.draw(move |f| {
            let blockchain_data = blockchain_data.lock().unwrap().to_vec();
            let addresses_data = addresses_data.lock().unwrap().to_vec();
            let times = times.lock().unwrap();
            let tabs_state = tabs_state.lock().unwrap();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(17),
                        Constraint::Percentage(17),
                        Constraint::Percentage(17),
                        Constraint::Percentage(17),
                        Constraint::Percentage(17),
                        Constraint::Percentage(5),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            let titles = tabs_state
                .titles
                .iter()
                .map(|title| Spans::from(vec![Span::raw(title)]))
                .collect::<Vec<Spans>>();
            let tabs = Tabs::new(titles)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Menu")
                        .title_alignment(Alignment::Center),
                )
                .select(tabs_state.current)
                .style(Style::default().fg(Color::Cyan))
                .highlight_style(
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .bg(Color::Black),
                );
            f.render_widget(tabs, chunks[0]);

            match tabs_state.current {
                0 => {
                    for i in 0..5 {
                        let paragraph = Paragraph::new("█".repeat(blockchain_data[i].1))
                            .style(Style::default().fg(Color::White))
                            .block(
                                Block::default()
                                    .title(format!(
                                        "node {} (Block height: {})",
                                        i, blockchain_data[i].1
                                    ))
                                    .borders(Borders::ALL),
                            )
                            .alignment(Alignment::Left)
                            .wrap(Wrap { trim: true });
                        f.render_widget(paragraph, chunks[i + 1]);
                    }

                    // Average time of block creation
                    let block_creation_avg = {
                        let mut avg = 0;
                        for block_time in times.iter() {
                            avg += block_time;
                        }
                        avg / 5
                    };

                    // 500 txs is the block size
                    let tps = {
                        if block_creation_avg <= 1 {
                            0
                        } else {
                            *block_size.lock().unwrap() / (block_creation_avg / 1000)
                        }
                    };

                    let paragraph = Paragraph::new(format!(
                        "Block time = {}ms | TPS = {}",
                        block_creation_avg, tps
                    ))
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, chunks[6]);
                }
                1 => {
                    for i in 0..5 {
                        let paragraph =
                            Paragraph::new(format!("Ammount -> {}", addresses_data[i].1))
                                .style(Style::default().fg(Color::White))
                                .block(
                                    Block::default()
                                        .title(format!(
                                            "node {} (Address: {})",
                                            i, addresses_data[i].0
                                        ))
                                        .borders(Borders::ALL),
                                )
                                .alignment(Alignment::Left)
                                .wrap(Wrap { trim: true });
                        f.render_widget(paragraph, chunks[i + 1]);
                    }
                }
                _ => {}
            }
        })?;

        if let Event::Input(event) = rx.recv().unwrap() {
            match event.code {
                KeyCode::Char('q') => {
                    disable_raw_mode().unwrap();
                    execute!(
                        terminal.backend_mut(),
                        LeaveAlternateScreen,
                        DisableMouseCapture
                    )
                    .unwrap();
                    terminal.show_cursor().unwrap();
                    std::process::exit(0);
                }
                KeyCode::Right => {
                    tabs.lock().unwrap().next();
                }
                KeyCode::Left => {
                    tabs.lock().unwrap().prev();
                }
                _ => {}
            }
        }
    }
}
