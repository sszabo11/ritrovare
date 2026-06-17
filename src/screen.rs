use std::{
    io::{BufWriter, Write, stdout},
    time::Duration,
};

use crate::{
    local::LocalDB,
    model::Model,
    spinners::{Spinner, SpinnerDots},
};
use anyhow::Result;
use colored::Colorize;
use crossterm::{
    QueueableCommand,
    cursor::{self, SetCursorStyle},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
        MouseEvent, MouseEventKind,
    },
    execute, queue,
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, size},
};
use log::info;
use ollama_rs::generation::chat::{ChatMessage, MessageRole};
use termimad::{Area, FmtText, MadSkin};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct Screen {
    screen_height: u16,
    screen_width: u16,
    input: String,
    output: SearchResult,
    prompt_state: PromptState,
    spinner: SpinnerDots,

    ui_offset: u16,

    messages: Vec<ChatMessage>,

    content_height: u16,
    is_streaming: bool,
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

pub enum AppEvent {
    Token(String),
    Done,
    SearchResult(SearchResult),
    Error(String),
    StartStreaming,
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
            ui_offset: 0,
            messages: Vec::new(),
            content_height: 0,
            is_streaming: false,
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        //let mut stdout = stdout();
        let mut stdout = BufWriter::new(stdout());
        execute!(stdout, EnterAlternateScreen)?;
        execute!(stdout, EnableMouseCapture)?;
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        let (tx, mut rx) = mpsc::channel::<AppEvent>(300);
        loop {
            self.render(&mut stdout)?;
            if event::poll(Duration::from_millis(80))? {
                let event = event::read()?;
                //if let Event::Mouse(mouse) = event {
                //    self.handle_mouse(mouse);
                //}
                match event {
                    Event::Key(key) => match self.handle_key(key, &tx) {
                        Action::Quit => {
                            break;
                        }
                        Action::None => {}
                    },
                    Event::Mouse(mouse) => self.handle_mouse(mouse),
                    _ => {}
                }
            }

            match rx.try_recv() {
                // if is_streaming doesnt work, add evebt START here:
                Ok(event) => match event {
                    AppEvent::Token(token) => {
                        log::info!("TOKEN: {}", token);
                        self.output.content.push_str(&token);
                    }
                    AppEvent::StartStreaming => {
                        self.is_streaming = true;
                    }
                    AppEvent::SearchResult(result) => {
                        self.prompt_state = PromptState::None;
                        self.input.clear();
                        self.messages
                            .push(ChatMessage::new(MessageRole::Assistant, result.content));
                        self.output.content.clear();
                        self.is_streaming = false;
                    }
                    AppEvent::Error(err) => {
                        log::error!("Error receiving stream content: {}", err)
                    }
                    _ => {}
                },
                Err(err) => match err {
                    mpsc::error::TryRecvError::Empty => {}
                    _ => {
                        log::error!("Failed to receive message: {:?}", err);
                    }
                },
            };
        }
        terminal::disable_raw_mode()?;
        execute!(stdout, LeaveAlternateScreen)?;
        execute!(stdout, DisableMouseCapture)?;
        Ok(())
    }

    fn render(&mut self, stdout: &mut impl Write) -> Result<()> {
        queue!(stdout, terminal::Clear(terminal::ClearType::All))?;

        self.draw_title(stdout)?;
        self.draw_status_bar(stdout)?;

        self.draw_input(stdout)?;

        self.draw_output(stdout)?;

        stdout.flush()?;

        Ok(())
    }

    fn draw_input(&mut self, stdout: &mut impl Write) -> Result<()> {
        let input_y = 6 + self.content_height + self.ui_offset;

        if is_loading(&self.prompt_state) {
            let frame = self.spinner.tick();
            queue!(
                stdout,
                cursor::MoveTo(2, input_y),
                Print(format!("{}  ", frame))
            )?;
        } else {
            queue!(
                stdout,
                cursor::MoveTo(2, input_y),
                Print(get_indicator(&self.prompt_state)),
            )?;
        }
        queue!(
            stdout,
            cursor::MoveTo(5, input_y),
            SetCursorStyle::BlinkingBlock,
            Print(&self.input)
        )?;
        Ok(())
    }
    fn draw_output(&mut self, stdout: &mut impl Write) -> Result<()> {
        //if !matches!(self.prompt_state, PromptState::Enter) {
        //    return Ok(());
        //};

        //let markdown = termimad::inline(&self.output.content);

        for (i, msg) in self.messages.iter().enumerate() {
            let prev_content_height = (0..i)
                .into_iter()
                .map(|i| {
                    let m = &self.messages[i];
                    let padding = if m.role == MessageRole::User { 0 } else { 5 };

                    render_markdown(&m.content, self.screen_width as usize, padding).len() as u16
                })
                .sum::<u16>();

            let padding = if msg.role == MessageRole::User { 0 } else { 5 };
            queue!(
                stdout,
                cursor::MoveTo(5, 6 + prev_content_height + self.ui_offset)
            )?;
            let lines = render_markdown(&msg.content, self.screen_width as usize, padding as usize);
            for line in &lines {
                queue!(stdout, Print(line), Print("\r\n"))?;
            }
            //log::info!("content_heighT: {}", self.content_height);
        }
        self.content_height = (1..self.messages.len())
            .into_iter()
            .map(|i| {
                let m = &self.messages[i];
                let padding = if m.role == MessageRole::User { 0 } else { 5 };

                render_markdown(&m.content, self.screen_width as usize, padding).len() as u16
            })
            .sum::<u16>();

        let prev_content_height = (0..self.messages.len())
            .into_iter()
            .map(|i| {
                let m = &self.messages[i];
                let padding = if m.role == MessageRole::User { 0 } else { 5 };

                render_markdown(&m.content, self.screen_width as usize, padding).len() as u16
            })
            .sum::<u16>();
        if self.is_streaming {
            queue!(
                stdout,
                cursor::MoveTo(0, 6 + prev_content_height + self.ui_offset)
            )?;

            let lines = render_markdown(&self.output.content, self.screen_width as usize, 5);
            for line in &lines {
                queue!(stdout, Print(line), Print("\r\n"))?;
            }
            //log::info!("content_heighT: {}", self.content_height);
            self.content_height = lines.len() as u16;
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

        let status = match self.prompt_state {
            PromptState::Enter => "Searching...".to_string().yellow(),
            PromptState::Done => "Done.".to_string().green(),
            PromptState::Generating => "Generating...".to_string().magenta(),
            PromptState::None => text.to_string().blue(),
        };
        queue!(
            stdout,
            cursor::MoveTo(
                self.screen_width - 1 - text.len() as u16,
                self.screen_height - 1
            ),
            PrintStyledContent(status)
        )?;

        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.ui_offset += 2;
            }
            MouseEventKind::ScrollDown => {
                self.ui_offset = self.ui_offset.saturating_sub(2);
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent, tx: &Sender<AppEvent>) -> Action {
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
                let msgs = self.messages.clone();
                self.messages
                    .push(ChatMessage::new(MessageRole::User, self.input.to_string()));
                self.input.clear();

                tokio::spawn(async move {
                    log::info!("START SEARCH");
                    // DO LLM CALL
                    match run_search(query, &msgs, &tx).await {
                        Ok(result) => {
                            tx.send(AppEvent::SearchResult(result))
                                .await
                                .expect("Failed to send query response");
                        }
                        Err(err) => {
                            log::error!("Failed to run search: {}", err);
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

pub async fn run_search(
    query: String,
    messages: &[ChatMessage],
    tx: &Sender<AppEvent>,
) -> Result<SearchResult> {
    let model = Model::new("qwen3.5:27b-q4_K_M");
    let local = LocalDB::new();

    let query_embedding = model.embed_query(&query).await?;
    let vector_res = local.search_by_vector(query_embedding, 5).await?;

    let ids = vector_res.iter().map(|row| row.0).collect();
    let history_data = local.get_tabs_from_ids(ids).await?;
    log::info!("FINISHED VECTOR SEARCH");

    //log::info!("\nhistory data: {:?}", history_data);

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

    let query_w_data = format!(
        "User query: '{}'\nBrowsing history: {}\n",
        query, history_txt
    );

    let tx_token = tx.clone();
    log::info!("START LLM");
    tx.send(AppEvent::StartStreaming).await?;
    let result = model
        .search(&query_w_data, messages, move |token| {
            let tx = tx_token.clone();
            async move {
                tx.send(AppEvent::Token(token)).await.ok();
            }
        })
        .await?;

    Ok(result)
}

fn format_timestamp(micros: i64) -> String {
    use chrono::DateTime;
    let secs = micros / 1_000_000;
    DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn render_markdown(text: &str, terminal_width: usize, padding: usize) -> Vec<String> {
    let content_width = terminal_width.saturating_sub(padding * 2);
    let mut skin = MadSkin::default();
    skin.paragraph.left_margin = 0;
    skin.paragraph.right_margin = 0;
    for h in skin.headers.iter_mut() {
        h.set_fg(crossterm::style::Color::Rgb {
            r: 114,
            g: 142,
            b: 255,
        });
    }

    let fmt_text = FmtText::from(&skin, text, Some(content_width));
    fmt_text
        .to_string()
        .lines()
        .map(|l| format!("{}{}", " ".repeat(padding), l))
        .collect()
}
