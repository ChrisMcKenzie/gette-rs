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

    #[error("client not set")]
    ClientNotSet,

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
/// use gette::getter;
/// use async_trait::async_trait;
///
/// pub struct mygetter;
/// #[async_trait]
/// impl getter for mygetter {
///     async fn get(&self, _dest: &str, _source: &str) -> result<(), gette::error> {
///       ok(())   
///     }
/// }
///```
///
/// the next step is to add it to the builder:
///
///```rust
/// use gette::RequestBuilder;
/// # use gette::Getter;
/// # use async_trait::async_trait;
///
/// # pub struct Mygetter;
/// # #[async_trait]
/// # impl Getter for Mygetter {
/// #     async fn get(&self, _dest: &str, _source: &str) -> Result<(), gette::Error> {
/// #       Ok(())   
/// #     }
/// # }
///
/// # tokio_test::block_on(async {
/// let b = RequestBuilder::builder().src("mygetter://test.txt".to_string())
///     .dest("test2.txt".to_string())
///     .add_getter("mygetter", Box::new(Mygetter))
///     .get()
///     .await
///     .unwrap();
/// # })
///```
#[async_trait]
pub trait Getter {
    async fn get(&self, dest: &str, source: &str) -> Result<(), Error>;
    async fn set_client(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

pub trait Decompressor {}

#[derive(Default, Debug)]
pub struct NoSrc;
#[derive(Default, Debug)]
pub struct Src(String);

#[derive(Default, Debug)]
pub struct NoDest;
#[derive(Default, Debug)]
pub struct Dest(String);

pub struct RequestBuilder<S, D> {
    src: S,
    dest: D,
    detectors: Vec<Box<dyn Detector>>,
    getters: HashMap<String, Box<dyn Getter + Send>>,
}

impl Default for RequestBuilder<NoSrc, NoDest> {
    fn default() -> Self {
        let mut getters: HashMap<String, Box<dyn Getter + Send>> = HashMap::new();
        getters.insert("file".to_string(), Box::new(getters::File));

        let s3 = getters::S3::default();
        getters.insert("s3".to_string(), Box::new(s3));

        Self {
            src: NoSrc,
            dest: NoDest,
            getters,
            detectors: vec![Box::new(detectors::File), Box::new(detectors::S3)],
        }
    }
}

impl RequestBuilder<NoSrc, NoDest> {
    pub fn builder() -> Self {
        Default::default()
    }
}

impl<D> RequestBuilder<NoSrc, D> {
    pub fn src(self, src: String) -> RequestBuilder<Src, D> {
        let Self {
            src: _,
            dest,
            detectors,
            getters,
        } = self;

        RequestBuilder {
            src: Src(src),
            dest,
            detectors,
            getters,
        }
    }
}

impl<S> RequestBuilder<S, NoDest> {
    pub fn dest(self, dest: String) -> RequestBuilder<S, Dest> {
        let Self {
            src,
            dest: _,
            detectors,
            getters,
        } = self;

        RequestBuilder {
            src,
            dest: Dest(dest),
            detectors,
            getters,
        }
    }
}

impl<S, D> RequestBuilder<S, D> {
    pub fn add_getter(mut self, name: &str, getter: Box<dyn Getter + Send>) -> Self {
        self.getters.insert(name.to_string(), getter);
        self
    }

    pub fn add_detector(mut self, detector: Box<dyn Detector>) -> Self {
        self.detectors.push(detector);
        self
    }
}

impl RequestBuilder<Src, Dest> {
    fn detect(&self) -> Result<String, Error> {
        let (is_force, _) = get_forced_proto(&self.src.0);

        if Url::parse(self.src.0.as_str()).is_ok() {
            return Ok(self.src.0.clone());
        }

        for d in self.detectors.iter() {
            let res = d.detect(&self.src.0)?;

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

        Err(Error::GetterNotFound(self.src.0.clone()))
    }

    pub async fn get(&self) -> Result<(), Error> {
        let src = self.detect()?;

        let (mut forced, src) = get_forced_proto(&src);

        let parsed_url = Url::parse(src)?;
        if forced.is_none() {
            forced = Some(parsed_url.scheme());
        }

        if let Some(getter) = self.getters.get(forced.unwrap()) {
            return getter.get(&self.dest.0, src).await;
        }

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

    #[tokio::test]
    async fn test_simple_detect() {
        let b = RequestBuilder::builder()
            .src("file://test.txt".to_string())
            .dest("test2.txt".to_string());
        let res = b.detect().unwrap();
        assert_eq!("file://test.txt", res);
    }

    #[tokio::test]
    async fn test_file_detect_without_proto() {
        let b = RequestBuilder::builder()
            .src("test.txt".to_string())
            .dest("test2.txt".to_string());

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
        let builder = RequestBuilder::builder()
            .src(source.to_string())
            .dest(dest.to_string());

        println!("src: {:?}\ndest: {:?}", builder.src, builder.dest);
        builder.get().await.unwrap();
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
