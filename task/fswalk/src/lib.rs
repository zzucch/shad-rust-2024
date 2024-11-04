#![forbid(unsafe_code)]

use std::{
    fs,
    io::{self, Result},
    path::Path,
};

////////////////////////////////////////////////////////////////////////////////

type Callback<'a> = dyn FnMut(&mut Handle) + 'a;

#[derive(Default)]
pub struct Walker<'a> {
    callbacks: Vec<Box<Callback<'a>>>,
}

impl<'a> Walker<'a> {
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut Handle) + 'a,
    {
        self.callbacks.push(Box::new(callback))
    }

    pub fn walk<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.walk_recursive(path.as_ref(), self.callbacks.len())
    }

    fn walk_recursive(&mut self, path: &Path, remaining_callbacks: usize) -> Result<()> {
        if remaining_callbacks == 0 {
            return Ok(());
        }

        let mut handle = if path.is_dir() {
            Handle::Dir(DirHandle::new(path))
        } else if path.is_file() {
            Handle::File(FileHandle::new(path))
        } else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Unsupported entity type",
            ));
        };

        let remaining_callbacks = self.run_callbacks(&mut handle, remaining_callbacks);

        match handle {
            Handle::Dir(dir_handle) => match dir_handle.content {
                None => Ok(()),
                Some(Ok(mut read_dir)) => read_dir.try_for_each(|entry| match entry {
                    Ok(entry) => self.walk_recursive(entry.path().as_path(), remaining_callbacks),
                    Err(error) => Err(error),
                }),
                Some(Err(error)) => Err(error),
            },
            Handle::File(file_handle) => match file_handle.content {
                None => Ok(()),
                Some(Ok(content)) => {
                    let mut content_handle = Handle::Content {
                        file_path: file_handle.path,
                        content: &content,
                    };

                    self.run_callbacks(&mut content_handle, remaining_callbacks);

                    Ok(())
                }
                Some(Err(error)) => Err(error),
            },
            _ => unreachable!(),
        }
    }

    fn run_callbacks(&mut self, handle: &mut Handle, remaining_callbacks: usize) -> usize {
        let mut skipped_callbacks = Vec::new();

        self.callbacks
            .iter_mut()
            .take(remaining_callbacks)
            .enumerate()
            .for_each(|(i, callback)| {
                callback(handle);

                let is_relevant = match handle {
                    Handle::Dir(dir_handle) => {
                        let descend = dir_handle.is_descent;
                        dir_handle.is_descent = false;

                        descend
                    }
                    Handle::File(file_handle) => {
                        let read = file_handle.is_read;
                        file_handle.is_read = false;

                        read
                    }
                    _ => true,
                };

                if !is_relevant {
                    skipped_callbacks.push(i);
                }
            });

        let mut remaining_callbacks = remaining_callbacks;

        skipped_callbacks.iter().rev().for_each(|&i| {
            self.callbacks.swap(i, remaining_callbacks - 1);
            remaining_callbacks -= 1;
        });

        remaining_callbacks
    }
}

////////////////////////////////////////////////////////////////////////////////

pub enum Handle<'a> {
    Dir(DirHandle<'a>),
    File(FileHandle<'a>),
    Content {
        file_path: &'a Path,
        content: &'a [u8],
    },
}

pub struct DirHandle<'a> {
    path: &'a Path,
    is_descent: bool,
    content: Option<Result<fs::ReadDir>>,
}

impl<'a> DirHandle<'a> {
    fn new(path: &'a std::path::Path) -> Self {
        Self {
            path,
            is_descent: false,
            content: None,
        }
    }

    pub fn descend(&mut self) {
        if self.content.is_none() {
            let read_dir = fs::read_dir(self.path);
            self.content = Some(read_dir);
        }

        self.is_descent = true;
    }

    pub fn path(&self) -> &Path {
        self.path
    }
}

pub struct FileHandle<'a> {
    path: &'a Path,
    is_read: bool,
    content: Option<Result<Vec<u8>>>,
}

impl<'a> FileHandle<'a> {
    fn new(path: &'a std::path::Path) -> Self {
        Self {
            path,
            is_read: false,
            content: None,
        }
    }

    pub fn read(&mut self) {
        if self.content.is_none() {
            let content = fs::read(self.path);
            self.content = Some(content);
        }

        self.is_read = true;
    }

    pub fn path(&self) -> &Path {
        self.path
    }
}
