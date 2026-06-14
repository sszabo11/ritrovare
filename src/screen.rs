use std::{
    io::{Write, stdout},
    time::Duration,
};

use crate::{
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
    prompt_state: PromptState,
    spinner: SpinnerDots,
    query_tx: Sender<QueryPayload>,
    query_rx: Receiver<QueryPayload>,
    model: Model,
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
pub struct QueryPayload {
    pub query: String,
}

impl Screen {
    pub fn new() -> Self {
        let (cols, rows) = size().expect("Failed to read size");
        let (tx, mut rx) = mpsc::channel::<QueryPayload>(100);

        Self {
            screen_height: rows,
            screen_width: cols,
            input: String::new(),
            prompt_state: PromptState::None,
            spinner: SpinnerDots::new(),
            query_tx: tx,
            query_rx: rx,
            model: Model::new("gemma4"),
        }
    }

    pub fn draw(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        loop {
            self.render(&mut stdout)?;
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    match self.handle_events(key) {
                        Action::Quit => {
                            break;
                        }
                        Action::None => {}
                    }
                }
            }

            if let Ok(msg) = self.query_rx.try_recv() {
                println!("Received {:?}", msg);
            } else {
                eprintln!("Failed to receive message");
            }
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
        if !matches!(self.prompt_state, PromptState::Enter) {
            return Ok(());
        };

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

    fn handle_events(&mut self, key: KeyEvent) -> Action {
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
                let tx = self.query_tx.clone();
                tokio::spawn(async move {
                    // DO LLM CALL
                    let result = run_search(query).await.unwrap();
                    println!("ya: {:?}", result);
                    tx.send(result)
                        .await
                        .expect("Failed to send query response");
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

pub async fn run_search(query: String) -> Result<QueryPayload> {
    Ok(QueryPayload {
        query: "".to_string(),
    })
}
