pub trait Spinner {
    fn new() -> Self;

    fn tick(&mut self) -> char;
}

pub struct Spinner1 {
    frames: Vec<char>,
    index: usize,
}

impl Spinner for Spinner1 {
    fn new() -> Self {
        Self {
            frames: vec!['|', '/', '-', '\\'],
            index: 0,
        }
    }

    fn tick(&mut self) -> char {
        let frame = self.frames[self.index];
        self.index = (self.index + 1) % self.frames.len();
        frame
    }
}

pub struct SpinnerDots {
    frames: Vec<char>,
    index: usize,
}

impl Spinner for SpinnerDots {
    fn new() -> Self {
        Self {
            //frames: vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            frames: vec!['◐', '◓', '◑', '◒'],
            index: 0,
        }
    }

    fn tick(&mut self) -> char {
        let frame = self.frames[self.index];
        self.index = (self.index + 1) % self.frames.len();
        frame
    }
}
