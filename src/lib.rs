use std::collections::HashMap;

use getters::file;
use regex::Regex;
use url::Url;

pub mod getters;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    // Http(reqwest::Error),
    // Git(git2::Error),
    #[error("source file not found")]
    SourceNotFound,

    #[error("getter for {0} not found")]
    GetterNotFound(String),

    #[error("destination path already exists and is not a symlink")]
    DestinationExists,
    #[error("destination could not be created")]
    DestinationNotCreated,

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("unknown error")]
    Unknown,
}

pub trait Detector {
    fn detect(&self, path: &str) -> Option<String>;
}

pub trait Getter {
    fn get(&self, dest: &str, source: &str) -> Result<(), Error>;
    fn copy(&self, dest: &str, source: &str) -> Result<(), Error>;
}

pub struct Builder {
    src: String,
    dest: String,
    detectors: Vec<Box<dyn Detector>>,
    getters: HashMap<String, Box<dyn Getter>>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            src: "".to_string(),
            dest: "".to_string(),
            getters: HashMap::from([]),
            detectors: vec![Box::new(file::File)],
        }
    }
}

impl Builder {
    pub fn new(src: &str, dest: &str) -> Self {
        Self {
            src: src.to_string(),
            dest: dest.to_string(),
            ..Default::default()
        }
    }

    pub fn add_getter(mut self, name: &str, getter: Box<dyn Getter>) -> Self {
        self.getters.insert(name.to_string(), getter);
        self
    }

    pub fn add_detector(mut self, detector: Box<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }

    fn detect(&self) -> Result<String, Error> {
        if self.src.is_empty() {
            return Err(Error::SourceNotFound);
        }

        let (is_force, _) = get_forced_proto(&self.src);

        if Url::parse(self.src.as_str()).is_ok() {
            return Ok(self.src.clone());
        }

        for d in self.detectors.iter() {
            let res = d.detect(&self.src);

            if res.is_none() {
                continue;
            }

            let src: &str = &res.unwrap();
            let (forced_proto, src) = get_forced_proto(src);

            if is_force.is_some() {
                return Ok(format!("{}+{}", is_force.unwrap(), src));
            } else if forced_proto.is_some() {
                return Ok(format!("{}+{}", forced_proto.unwrap(), src));
            }

            return Ok(src.to_string());
        }

        Err(Error::GetterNotFound(self.src.clone()))
    }

    pub fn get(&self) -> Result<(), Error> {
        let src = self.detect()?;

        let (mut forced, src) = get_forced_proto(&src);

        let parsed_url = Url::parse(src)?;
        if forced.is_none() {
            forced = Some(parsed_url.scheme());
        }

        if let Some(getter) = self.getters.get(forced.unwrap()) {
            return getter.get(&self.dest, src);
        }

        Ok(())
    }

    pub fn copy(self) -> Result<(), Error> {
        Ok(())
    }
}

fn get_forced_proto(v: &str) -> (Option<&str>, &str) {
    if let Some(re) = Regex::new(r"^([A-Za-z0-9]+)\+(.*)$").unwrap().captures(v) {
        return (
            Some(re.get(1).unwrap().as_str()),
            re.get(2).unwrap().as_str(),
        );
    }

    (None, v)
}

#[cfg(test)]
mod tests {

    use std::{
        fs::{self, File},
        io::Write,
    };

    use super::*;

    #[test]
    fn test_simple_detect() {
        let b = Builder::new("file://test.txt", "test2.txt");
        let res = b.detect().unwrap();
        assert_eq!("file://test.txt", res);
    }

    #[test]
    fn test_file_detect_without_proto() {
        let b = Builder::new("test.txt", "test2.txt");
        let res = b.detect().unwrap();
        assert_eq!("file://test.txt", res);
    }

    #[test]
    fn test_get_call() {
        let source = "./test.txt";
        let dest = "./test2.txt";
        let mut f = File::create(source).unwrap();
        f.write_all("test".as_bytes()).unwrap();
        let b = Builder::new("file://./test.txt", dest).add_getter("file", Box::new(file::File));
        b.get().unwrap();
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
