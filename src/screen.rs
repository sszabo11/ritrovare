use std::{
    io::{Write, stdout},
    time::Duration,
};

use crate::{
    local::LocalDB,
    model::Model,
    spinners::{Spinner, SpinnerDots},
};
use anyhow::Result;
use crossterm::{
    QueueableCommand,
    cursor::{self, SetCursorStyle},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute, queue,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct Screen {
    screen_height: u16,
    screen_width: u16,
    input: String,
    output: SearchResult,
    prompt_state: PromptState,
    spinner: SpinnerDots,
}

#[derive(Debug)]
pub enum PromptState {
    Generating,
    None,
    Enter,
    Done,
}

pub enum Action {
    None,
    Quit,
}

#[derive(Debug)]
pub struct SearchResult {
    pub content: String,
}
impl Default for SearchResult {
    fn default() -> Self {
        Self {
            content: String::new(),
        }
    }
}

impl Screen {
    pub fn new() -> Self {
        let (cols, rows) = size().expect("Failed to read size");

        Self {
            screen_height: rows,
            screen_width: cols,
            input: String::new(),
            output: SearchResult::default(),
            prompt_state: PromptState::None,
            spinner: SpinnerDots::new(),
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        let (tx, mut rx) = mpsc::channel::<SearchResult>(100);
        loop {
            self.render(&mut stdout)?;
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    match self.handle_events(key, &tx) {
                        Action::Quit => {
                            break;
                        }
                        Action::None => {}
                    }
                }
            }

            match rx.try_recv() {
                Ok(result) => {
                    self.prompt_state = PromptState::None;
                    self.output = result;
                }
                Err(err) => match err {
                    mpsc::error::TryRecvError::Empty => {}
                    _ => {
                        log::info!("Failed to receive message: {:?}", err);
                    }
                },
            };
        }
        terminal::disable_raw_mode()?;
        execute!(stdout, LeaveAlternateScreen)?;
        Ok(())
    }

    fn render(&mut self, stdout: &mut impl Write) -> Result<()> {
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        self.draw_title(stdout)?;
        self.draw_status_bar(stdout)?;

        self.draw_input(stdout)?;

        self.draw_output(stdout)?;

        stdout.flush()?;

        Ok(())
    }

    fn draw_input(&mut self, stdout: &mut impl Write) -> Result<()> {
        if is_loading(&self.prompt_state) {
            let frame = self.spinner.tick();
            queue!(stdout, cursor::MoveTo(2, 6), Print(format!("{}  ", frame)))?;
        } else {
            queue!(
                stdout,
                cursor::MoveTo(2, 6),
                Print(get_indicator(&self.prompt_state)),
            )?;
        }
        queue!(
            stdout,
            cursor::MoveTo(5, 6),
            SetCursorStyle::BlinkingBlock,
            Print(&self.input)
        )?;
        Ok(())
    }
    fn draw_output(&self, stdout: &mut impl Write) -> Result<()> {
        //if !matches!(self.prompt_state, PromptState::Enter) {
        //    return Ok(());
        //};

        if !self.output.content.is_empty() {
            queue!(stdout, cursor::MoveTo(5, 8), Print(&self.output.content))?;
        }
        Ok(())
    }

    fn draw_title(&self, stdout: &mut impl Write) -> Result<()> {
        let title = "\r\n┳┓•\r\n┣┫┓╋┏┓┏┓┓┏┏┓┏┓┏┓\r\n┛┗┗┗┛ ┗┛┗┛┗┻┛ ┗ ";

        queue!(
            stdout,
            cursor::MoveTo(6, 0),
            Print(title.with(Color::Rgb {
                r: 114,
                g: 142,
                b: 255
            }))
        )?;
        Ok(())
    }

    fn draw_status_bar(&self, stdout: &mut impl Write) -> Result<()> {
        let text = "Last synced: 3 minutes ago.";

        queue!(
            stdout,
            cursor::MoveTo(
                self.screen_width - 1 - text.len() as u16,
                self.screen_height - 1
            ),
            PrintStyledContent(text.blue())
        )?;
        Ok(())
    }

    fn handle_events(&mut self, key: KeyEvent, tx: &Sender<SearchResult>) -> Action {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                return Action::Quit;
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Enter => {
                self.prompt_state = PromptState::Enter;
                let query = self.input.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    // DO LLM CALL
                    match run_search(query).await {
                        Ok(result) => {
                            tx.send(result)
                                .await
                                .expect("Failed to send query response");
                        }
                        Err(err) => {
                            log::info!("Failed to run search: {}", err);
                        }
                    };
                });
            }
            KeyCode::Esc => {
                self.prompt_state = PromptState::None;
            }

            KeyCode::Char('w') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.input = match self.input.rsplit_once(' ') {
                    Some((remaining, _)) => remaining,
                    None => "",
                }
                .to_string();
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            // handle other key events
            _ => {}
        };
        Action::None
    }
}

//        let title = "
//┳┓•
//┣┫┓╋┏┓┏┓┓┏┏┓┏┓┏┓
//┛┗┗┗┛ ┗┛┗┛┗┻┛ ┗ "
//            .truecolor(114, 142, 255);
//
//

fn get_indicator(state: &PromptState) -> String {
    match state {
        PromptState::None => "✦  ",
        PromptState::Generating => "✦  ",
        PromptState::Done => "✓  ",
        PromptState::Enter => "✦  ",
    }
    .to_string()
}

fn is_loading(state: &PromptState) -> bool {
    match state {
        PromptState::None => false,
        PromptState::Generating => true,
        PromptState::Done => false,
        PromptState::Enter => true,
    }
}

pub async fn run_search(query: String) -> Result<SearchResult> {
    let model = Model::new("gemma4");
    let local = LocalDB::new();

    let query_embedding = model.embed_query(&query).await?;
    let vector_res = local.search_by_vector(query_embedding, 20).await?;
    log::info!("vector res: {:?}", vector_res);

    let ids = vector_res.iter().map(|row| row.0).collect();
    log::info!("\nids: {:?}", ids);
    let history_data = local.get_tabs_from_ids(ids).await?;

    log::info!("\nhistory data: {:?}", history_data);

    let history_txt = history_data
        .iter()
        .map(|tab| {
            format!(
                "- Title: {}\n  URL: {}\n  Visited: {} times, last on {}\n  Time spent: {}s\n",
                tab.title,
                tab.url,
                tab.visit_count,
                format_timestamp(tab.last_visit_date),
                tab.total_view_time,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    log::info!("\nhistory txt: {}", history_txt);

    let query_w_data = format!("User query: {}\nBrowsing history: {}\n", query, history_txt);
    let result = model.search(&query_w_data).await?;

    Ok(result)
}

fn format_timestamp(micros: i64) -> String {
    use chrono::{DateTime, Utc};
    let secs = micros / 1_000_000;
    DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
