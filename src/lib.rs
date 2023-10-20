use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use url::Url;

pub mod detectors;
pub mod getters;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid url: {0}, reason: {1}")]
    InvalidUrl(String, String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

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

    #[error(transparent)]
    Unknown(#[from] Box<dyn std::error::Error>),
}

pub trait Detector {
    fn detect(&self, path: &str) -> Result<Option<String>, Error>;
}

/// Getter trait
/// Implement this trait to add a new getter
///
/// ## Extending Gette
///
/// Gette is designed to be extensible. You can add your own getters by implementing this trait.
/// the first step is to create a struct that implements this trait:
/// 
/// ```ignore
/// use gette::Getter;
/// use async_trait::async_trait;
///
/// pub struct MyGetter;
/// #[async_trait]
/// impl Getter for MyGetter {
///     async fn get(&self, _dest: &str, _source: &str) -> Result<(), gette::Error> {
///       Ok(())   
///     }
/// }
///```
///
/// the next step is to add it to the builder:
///
///```ignore
/// use gette::Builder;
///
/// let b = Builder::new("mygetter://test.txt", "test2.txt")
///     .add_getter("mygetter", MyGetter)
///     .get()
///     .await
///     .unwrap();
///```
#[async_trait]
pub trait Getter {
    async fn get(&self, dest: &str, source: &str) -> Result<(), Error>;
}

pub trait Decompressor {}

pub struct Builder {
    src: String,
    dest: String,
    detectors: Vec<Box<dyn Detector>>,
    getters: HashMap<String, Box<dyn Getter>>,
}

impl Default for Builder {
    fn default() -> Self {
        let mut getters: HashMap<String, Box<dyn Getter>> = HashMap::new();
        getters.insert("file".to_string(), Box::new(getters::File));
        let s3: Box<getters::S3> = Box::default();
        getters.insert("s3".to_string(), s3);

        Self {
            src: "".to_string(),
            dest: "".to_string(),
            getters,
            detectors: vec![Box::new(detectors::File), Box::new(detectors::S3)],
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
            let res = d.detect(&self.src)?;

            println!("res: {:?}", res);

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

    pub async fn get(&self) -> Result<(), Error> {
        let src = self.detect()?;

        let (mut forced, src) = get_forced_proto(&src);

        let parsed_url = Url::parse(src)?;
        if forced.is_none() {
            forced = Some(parsed_url.scheme());
        }

        if let Some(getter) = self.getters.get(forced.unwrap()) {
            return getter.get(&self.dest, src).await;
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
        env,
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
        let p = env::current_dir().unwrap().join("test.txt");
        assert_eq!(format!("file://{}", p.to_str().unwrap()), res);
    }

    #[tokio::test]
    async fn test_get_call() {
        let source = "./test-get-call.txt";
        let dest = "./test-get-call-destination.txt";
        let mut f = File::create(source).unwrap();

        f.write_all("test".as_bytes()).unwrap();
        Builder::new(source, dest).get().await.unwrap();
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
