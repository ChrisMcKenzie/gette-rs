// use serde::Deserialize;
// use url::Position;

pub mod file;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    // Http(reqwest::Error),
    // Git(git2::Error),
    SourceNotFound,

    DestinationExists,
    DestinationNotCreated,

    Unknown,
}

pub trait Getter {
    fn get(&self, dest: &str, source: &str) -> Result<(), Error>;
    fn copy(&self, dest: &str, source: &str) -> Result<(), Error>;
    fn detect(&self, path: &str) -> bool;
}

// #[derive(Debug, Clone)]
// pub enum Path {
//     File(String),
//     Git(String),
//     Http(String),
//     Https(String),
//     Unknown(String),
// }
//
// impl<'de> Deserialize<'de> for Path {
//     fn deserialize<D>(deserializer: D) -> Result<Path, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let s = String::deserialize(deserializer)?;
//         Ok(From::from(s))
//     }
// }
//
// impl From<String> for Path {
//     fn from(value: String) -> Self {
//         let parsed = url::Url::parse(&value);
//         match parsed {
//             Ok(url) => {
//                 let path = url[Position::BeforeUsername..].to_string();
//                 // TODO(ChrisMcKenzie): this is fairly naive, and should also detect type based on
//                 // the rest of the url i.e. github.com is likely a git type
//                 match url.scheme() {
//                     "file" => Path::File(path),
//                     "git" => Path::Git(path),
//                     "http" => Path::Http(path),
//                     "https" => Path::Https(path),
//                     _ => Path::Unknown(value),
//                 }
//             }
//             Err(_) => Path::File(value),
//         }
//     }
// }

pub struct Builder {
    getters: Vec<Box<dyn Getter>>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            getters: Vec::new(),
        }
    }

    pub fn add_getter(&mut self, getter: Box<dyn Getter>) {
        self.getters.push(getter);
    }

    fn detect(&self, path: &str) -> Option<&dyn Getter> {
        for getter in self.getters.iter() {
            if getter.detect(path) {
                return Some(getter.as_ref());
            }
        }
        None
    }

    pub fn get(&self, dest: &str, source: &str) -> Result<(), Error> {
        if let Some(getter) = self.detect(source) {
            getter.get(dest, source)
        } else {
            Err(Error::Unknown)
        }
    }

    pub fn copy(&self, dest: &str, source: &str) -> Result<(), Error> {
        if let Some(getter) = self.detect(source) {
            getter.copy(dest, source)
        } else {
            Err(Error::Unknown)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fails_when_no_detector() {
        let dest = "/tmp/test.txt";
        let source = "not_found:///tmp/test.txt";
        let builder = Builder::new();
        assert!(builder.get(dest, source).is_err());
    }

    #[test]
    fn custom_type_def() {
        let source = "mystore://test.txt";

        struct MyGetter;
        impl Getter for MyGetter {
            fn get(&self, _dest: &str, _source: &str) -> Result<(), Error> {
                Ok(())
            }

            fn detect(&self, path: &str) -> bool {
                let u = url::Url::parse(path);
                if let Ok(u) = u {
                    if u.scheme() == "mystore" {
                        return true;
                    }
                }
                false
            }

            fn copy(&self, _dest: &str, _source: &str) -> Result<(), Error> {
                Ok(())
            }
        }

        let mut builder = Builder::new();
        builder.add_getter(Box::new(MyGetter));
        assert!(builder.get("/tmp/test.txt", source).is_ok());
    }
}
