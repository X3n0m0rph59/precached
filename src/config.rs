#[derive(Debug)]
pub struct Config {
    verbosity: u8
}

impl Config {
    pub fn new(args: &Vec<String>) -> Config {
        Config { verbosity: 1 }
    }
}
