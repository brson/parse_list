//! Parse files and lists of stringified things into lists of thingified things.
//!
//! That is, if you've got something `Read`-y or `Iterator`-y over `u8`s,
//! `String`s, or `str`s, and a type that implements `FromStr`, you can also
//! have an `Iterator` of that type of thing.
//!
//! Particularly designed to parse files of newline-separated things,
//! like these git integers:
//!
//! ```no-test
//! 0
//! 1
//! 2
//! 3
//! 4
//! ```
//!
//! Load your ints with ease:
//!
//! ```rust,ignore
//! // Create the file of test data
//! use std::fs;
//! let tmp_dir = TempDir::new("tmp").unwrap();
//! let file_path = tmp_dir.path().join("list");
//! fs::write(&file_path, "0\n1\n2\n3\n4").unwrap();
//!
//! // Load from file. Note that each element could result in an individual
//! // I/O or parse error. Here those are converted into a single `Result<Vec<u32>, _>`.
//! let v = from_file_lines(&file_path);
//! let v: Vec<Result<u32, _>> = v.unwrap().collect();
//! let v: Result<Vec<u32>, _> = v.into_iter().collect();
//! let v = v.unwrap();
//! assert!(v == vec![0, 1, 2, 3, 4]);
//! ```
//!
//! Besides parsing from a newline-separated file there are also functions for
//! parsing from various traits, including iterators.
//!
//! ## Tips
//!
//! To convert from an iterator of `Result` to a `Result` of `Vec` use `collect`
//! with a `Result<Vec<_>>` type annotation:
//!
//! ```rust
//! use big_s::S;
//!
//! let a = vec![Ok(S("0")), Ok(S("1")), Ok(S("2"))];
//! let b: Vec<Result<u32, _>> = parse_list::from_iter(a.into_iter()).collect();
//! let b: Result<Vec<u32>, _> = b.into_iter().collect();
//! let b = b.unwrap();
//!
//! assert!(b == vec![0, 1, 2]);
//! ```
//!
//! To ignore errors parsing any particular list entry, use
//! `filter_map(Result::ok)`:
//!
//! ```rust
//! use big_s::S;
//! use std::io;
//! use std::num::ParseIntError;
//!
//! let e: io::Error = io::Error::from(io::ErrorKind::NotFound);
//! let a: Vec<Result<String, io::Error>> = vec![Ok(S("0")), Err(e), Ok(S("2"))];
//! let b: Vec<u32> = parse_list::from_iter(a.into_iter())
//!     .filter_map(Result::ok).collect();
//!
//! assert!(b.len() == 2);
//! assert!(b[0] == 0);
//! assert!(b[1] == 2);
//! ```

use std::marker::PhantomData;
use std::fmt::{self, Display};
use std::error::Error;
use std::iter::{Iterator, Filter};
use std::fs::File;
use std::io::{self, Read, BufReader, BufRead, Lines};
use std::path::Path;
use std::str::FromStr;

pub fn from_file_lines<T>(p: &Path) -> Result<ParseListIterator<T, Filter<Lines<BufReader<File>>, fn(&Result<String, io::Error>) -> bool>>, io::Error>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static {
    let f = File::open(p)?;
    Ok(from_read_lines(f))
}

pub fn from_read_lines<T, R>(r: R) -> ParseListIterator<T, Filter<Lines<BufReader<R>>, fn(&Result<String, io::Error>) -> bool>>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static,
      R: Read {
    let r: BufReader<R> = BufReader::new(r);
    from_bufread_lines(r)
}

pub fn from_bufread_lines<T, B>(b: B) -> ParseListIterator<T, Filter<Lines<B>, fn(&Result<String, io::Error>) -> bool>>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static,
      B: BufRead {

    fn nonblank(lr: &Result<String, io::Error>) -> bool {
        let trimmed = lr.as_ref().map(|l| !l.trim().is_empty());
        let nonblank = trimmed.unwrap_or(true);
        nonblank
    }

    let without_blanks = b.lines().filter(nonblank as fn(&Result<String, io::Error>) -> bool);

    from_iter(without_blanks)
}

// TODO: abstract io::Error

pub fn from_iter<T, I>(i: I) -> ParseListIterator<T, I>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static,
      I: Iterator<Item = Result<String, io::Error>> {
    ParseListIterator::<T, I>(i, PhantomData)
}

pub struct ParseListIterator<T, I> (I, PhantomData<T>)
where T: FromStr,
      T::Err: Error + Send + Sync + 'static,
      I: Iterator<Item = Result<String, io::Error>>;

impl<T, I> Iterator for ParseListIterator<T, I>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static,
      I: Iterator<Item = Result<String, io::Error>>
{

    type Item = Result<T, ParseListError<T::Err>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(string_result_to_item_result)
    }
}

fn string_result_to_item_result<T>(v: Result<String, io::Error>) -> Result<T, ParseListError<T::Err>>
where T: FromStr,
      T::Err: Error + Send + Sync + 'static {
    match v {
        Ok(v) => {
            match str::parse(&v) {
                Ok(v) => Ok(v),
                Err(e) => Err(ParseListError::Parse(e))
            }
        }
        Err(e) => Err(ParseListError::Io(e))
    }
}

#[derive(Debug)]
pub enum ParseListError<TE>
where TE: Error + Send + Sync + 'static {
    Io(io::Error),
    Parse(TE),
}

impl<TE> Error for ParseListError<TE>
where TE: Error + Send + Sync + 'static { }

impl<TE> Display for ParseListError<TE>
where TE: Error + Send + Sync + 'static {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseListError::Io(e) => Display::fmt(e, f),
            ParseListError::Parse(e) => Display::fmt(e, f),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use big_s::S;
    use super::*;

    #[test]
    fn from_iter_vec() {
        let a = vec![Ok(S("0")), Ok(S("1")), Ok(S("2"))];
        let b: Vec<Result<u32, _>> = from_iter(a.into_iter()).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    // What happens with [Ok, Err, Ok]?
    #[test]
    fn from_iter_vec_fail_middle() {
        use std::num::ParseIntError;
        let e: io::Error = io::Error::from(io::ErrorKind::NotFound);
        let a: Vec<Result<String, io::Error>> = vec![Ok(S("0")), Err(e), Ok(S("2"))];
        let b: Vec<Result<u32, ParseListError<ParseIntError>>> = from_iter(a.into_iter()).collect();
        assert!(b.len() == 3);
        assert!(b[0].as_ref().unwrap() == &0);
        assert!(b[1].is_err());
        assert!(b[2].as_ref().unwrap() == &2);
    }


    #[test]
    fn from_iter_vec_ignore_errors() {
        let e: io::Error = io::Error::from(io::ErrorKind::NotFound);
        let a: Vec<Result<String, io::Error>> = vec![Ok(S("0")), Err(e), Ok(S("2"))];
        let b: Vec<u32> = from_iter(a.into_iter())
            .filter_map(Result::ok).collect();
        assert!(b.len() == 2);
        assert!(b[0] == 0);
        assert!(b[1] == 2);
    }

    #[test]
    fn from_bufread_lines_slice() {
        let a = "0\n1\n2".as_bytes();
        let b: Vec<Result<u32, _>> = from_bufread_lines(a).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_bufread_lines_slice_fail_middle() {
        let a = "0\nboop\n2".as_bytes();
        let b: Vec<Result<u32, _>> = from_bufread_lines(a).collect();
        assert!(b.len() == 3);
        assert!(b[0].as_ref().unwrap() == &0);
        assert!(b[1].is_err());
        assert!(b[2].as_ref().unwrap() == &2);
    }

    #[test]
    fn from_bufread_lines_cursor() {
        use std::io::Cursor;
        let a = Cursor::new("0\n1\n2".as_bytes());
        let b: Vec<Result<u32, _>> = from_bufread_lines(a).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_read_lines_slice() {
        let a = "0\n1\n2".as_bytes();
        let b: Vec<Result<u32, _>> = from_read_lines(a).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_read_lines_cursor() {
        use std::io::Cursor;
        let a = Cursor::new("0\n1\n2".as_bytes());
        let b: Vec<Result<u32, _>> = from_read_lines(a).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_read_lines_file() {
        use std::fs;
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        fs::write(&file_path, "0\n1\n2").unwrap();
        let f = File::open(file_path).unwrap();
        let b: Vec<Result<u32, _>> = from_read_lines(f).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_read_lines_file_slice() {
        use std::fs;
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        fs::write(&file_path, "0\n1\n2").unwrap();
        let f = File::open(file_path).unwrap();
        let b: Vec<Result<u32, _>> = from_read_lines(&f).collect();
        let b: Result<Vec<u32>, _> = b.into_iter().collect();
        let b = b.unwrap();
        assert!(b == vec![0, 1, 2]);
    }

    #[test]
    fn from_file_lines_success() {
        use std::fs;
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        fs::write(&file_path, "0\n1\n2\n3\n4").unwrap();

        let v = from_file_lines(&file_path);
        let v: Vec<Result<u32, _>> = v.unwrap().collect();
        let v: Result<Vec<u32>, _> = v.into_iter().collect();
        let v = v.unwrap();
        assert!(v == vec![0, 1, 2, 3, 4]);
    }
    
    #[test]
    fn from_file_lines_success_min_annotations() {
        use std::fs;
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        fs::write(&file_path, "0\n1\n2\n3\n4").unwrap();

        let v = from_file_lines(&file_path);
        let v: Vec<_> = v.unwrap().collect();
        let v: Result<Vec<u32>, _> = v.into_iter().collect();
        let v = v.unwrap();
        assert!(v == vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn from_file_lines_ignore_errors() -> Result<(), io::Error> {
        use std::fs;
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        fs::write(&file_path, "0\n1\n2\n3\n4").unwrap();
        let b: Vec<u32> = from_file_lines(&file_path)?
            .filter_map(Result::ok).collect();
        assert!(b == vec![0, 1, 2, 3, 4]);
        Ok(())
    }
    
    #[test]
    fn from_file_lines_not_found() {
        let tmp_dir = TempDir::new("tmp").unwrap();
        let file_path = tmp_dir.path().join("list");
        let b = from_file_lines::<u32>(&file_path);
        assert!(b.is_err());
    }
}

