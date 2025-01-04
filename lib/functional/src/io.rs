use crate::functor::Functor;
use std::fs;
use std::io::{BufWriter, Write};

pub enum IO<'a, A> {
    Return(A),
    Suspend(Box<dyn FnOnce() -> IO<'a, A> + 'a>),
    Error(String),
}

impl<'a, A: 'a> Functor<'a, A> for IO<'a, A> {
    type Unwrapped = A;
    type Wrapped<B: 'a> = IO<'a, B>;

    fn map<F, B>(self, map_fn: F) -> IO<'a, B>
    where
        B: 'a,
        F: FnOnce(A) -> B + 'a,
    {
        match self {
            IO::Return(a) => IO::Return(map_fn(a)),
            IO::Suspend(a) => IO::Suspend(Box::new(move || a().map(map_fn))),
            IO::Error(e) => IO::Error(e),
        }
    }

    fn flat_map<F, B>(self, map_fn: F) -> IO<'a, B>
    where
        B: 'a,
        F: FnOnce(A) -> IO<'a, B> + 'a,
    {
        match self {
            IO::Return(a) => map_fn(a),
            IO::Suspend(a) => IO::Suspend(Box::new(move || a().flat_map(map_fn))),
            IO::Error(e) => IO::Error(e),
        }
    }
}

impl<'a, A> IO<'a, A> {
    pub fn read_file(filename: &'a str) -> IO<String> {
        IO::Suspend(Box::new(move || match fs::read_to_string(filename) {
            Ok(content) => IO::Return(content),
            Err(e) => IO::Error(e.to_string()),
        }))
    }

    pub fn write_file(filename: &'a str, content: String) -> IO<()> {
        IO::Suspend(Box::new(move || match fs::File::create(filename) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                match writer.write_all(content.as_bytes()) {
                    Ok(_) => IO::Return(()),
                    Err(e) => IO::Error(e.to_string()),
                }
            }
            Err(e) => IO::Error(e.to_string()),
        }))
    }

    pub fn unsafe_run(self) -> Result<A, String> {
        match self {
            IO::Return(a) => Ok(a),
            IO::Suspend(a) => a().unsafe_run(),
            IO::Error(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::file;

    #[test]
    fn file_reader() {
        let file_io = IO::<String>::read_file(file!());
        let result = file_io.map(|content| content.to_uppercase()).unsafe_run();
        assert_eq!(true, result.unwrap().starts_with("USE"))
    }
}
