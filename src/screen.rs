use anyhow::Result;
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::Constraint;
use ratatui::{layout::Layout, widgets::Block};

pub struct Screen {}

impl Screen {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(&mut self) -> Result<()> {
        ratatui::run(|mut terminal| {
            loop {
                terminal.draw(|frame| self.render(frame))?;
                let quit = self.handle_events()?;
                if self.should_quit()? || quit {
                    break Ok(());
                }
            }
        })
    }

    fn render(&self, frame: &mut ratatui::Frame) {
        use Constraint::{Fill, Length, Min};

        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [title_area, main_area, status_area] = vertical.areas(frame.area());
        let horizontal = Layout::horizontal([Fill(1); 2]);
        let [left_area, right_area] = horizontal.areas(main_area);

        let title = r"
┳┓•             
┣┫┓╋┏┓┏┓┓┏┏┓┏┓┏┓
┛┗┗┗┛ ┗┛┗┛┗┻┛ ┗ 
            "
        .truecolor(114, 142, 255);
    }

    fn should_quit(&self) -> Result<bool> {
        Ok(false)
    }

    fn handle_events(&mut self) -> Result<bool> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') => return Ok(true),
                // handle other key events
                _ => {}
            },
            // handle other events
            _ => {}
        }
        Ok(false)
    }
}
