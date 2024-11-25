#![forbid(unsafe_code)]

use rayon::prelude::*;
use std::{
    fs::{read_dir, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub path: PathBuf,
    pub line: String,
    pub line_number: usize,
}

#[derive(Debug)]
pub struct Error {
    pub path: PathBuf,
    pub error: std::io::Error,
}

pub enum Event {
    Match(Match),
    Error(Error),
}

pub fn run<P: AsRef<Path>>(path: P, pattern: &str) -> Vec<Event> {
    let path = path.as_ref();
    process(path, pattern)
}

fn process(path: &Path, pattern: &str) -> Vec<Event> {
    if path.is_file() {
        process_file(path, pattern)
    } else {
        process_directory(path, pattern)
    }
}

fn process_file(path: &Path, pattern: &str) -> Vec<Event> {
    match File::open(path) {
        Ok(file) => BufReader::new(file)
            .lines()
            .enumerate()
            .flat_map(|(line_number, line)| match line {
                Ok(line) if line.contains(pattern) => Some(Event::Match(Match {
                    path: path.to_path_buf(),
                    line,
                    line_number: line_number + 1,
                })),
                Ok(_) => None,
                Err(err) => Some(Event::Error(Error {
                    path: path.to_path_buf(),
                    error: err,
                })),
            })
            .collect(),
        Err(err) => vec![Event::Error(Error {
            path: path.to_path_buf(),
            error: err,
        })],
    }
}

fn process_directory(path: &Path, pattern: &str) -> Vec<Event> {
    match read_dir(path) {
        Ok(read_dir) => read_dir
            .filter_map(Result::ok)
            .par_bridge()
            .flat_map(|dir_entry| process(&dir_entry.path(), pattern))
            .collect(),
        Err(err) => vec![Event::Error(Error {
            path: path.to_path_buf(),
            error: err,
        })],
    }
}
