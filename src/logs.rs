use simplelog::*;
use std::fs::File;

pub fn init_logging() {
    CombinedLogger::init(vec![WriteLogger::new(
        log::LevelFilter::Debug,
        Config::default(),
        File::create("ritrovare.log").unwrap(),
    )])
    .unwrap();
}
