use crate::Error;
use std::{fs, path::Path};
use url::{Position, Url};

pub struct File;

impl crate::Detector for File {
    fn detect(&self, path: &str) -> Option<String> {
        Some(format!("file://{}", path).to_string())
    }
}

impl crate::Getter for File {
    fn get(&self, dest: &str, source: &str) -> Result<(), crate::Error> {
        self.get(dest, source)
    }

    fn copy(&self, _dest: &str, _source: &str) -> Result<(), crate::Error> {
        Ok(())
    }
}

impl File {
    #[cfg(target_family = "unix")]
    fn get(&self, dest: &str, source: &str) -> Result<(), crate::Error> {
        let u = Url::parse(source)?;

        // validate source
        let source = Path::new(&u[Position::BeforeUsername..]);
        let dest = Path::new(dest);
        println!("{:?}", source);

        if !source.exists() {
            return Err(Error::SourceNotFound);
        }

        if dest.exists() {
            let meta = fs::symlink_metadata(dest).map_err(Error::Io)?.file_type();
            if !meta.is_symlink() {
                return Err(Error::DestinationExists);
            }

            fs::remove_file(dest).map_err(Error::Io)?
        }

        fs::create_dir_all(dest.parent().unwrap()).map_err(|_| Error::DestinationNotCreated)?;

        std::os::unix::fs::symlink(source, dest).map_err(Error::Io)?;

        Ok(())
    }

    #[cfg(target_family = "windows")]
    fn get(&self, dest: &str, source: &str) -> Result<(), crate::Error> {
        let u = Url::parse(source)?;

        // validate source
        let source = Path::new(&u[Position::BeforeUsername..]);
        let dest = Path::new(dest);

        if !source.exists() {
            return Err(Error::SourceNotFound);
        }

        if dest.exists() {
            let meta = fs::symlink_metadata(dest).map_err(Error::Io)?.file_type();
            if !meta.is_symlink() {
                return Err(Error::DestinationExists);
            }
            fs::remove_file(dest).map_err(crate::Error::Io)?
        }

        fs::create_dir_all(dest.parent().unwrap()).map_err(|_| Error::DestinationNotCreated)?;

        if source.is_dir() {
            std::os::windows::fs::symlink_dir(source, dest).map_err(crate::Error::Io)?;
        } else {
            std::os::windows::fs::symlink_file(source, dest).map_err(crate::Error::Io)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Read, Write},
    };

    use super::*;

    #[test]
    fn test_get_file_from_tmp() {
        let source = "./test-1.txt";
        let mut f = File::create(source).unwrap();
        f.write_all("test".as_bytes()).unwrap();

        let dest = "./test-2.txt";

        let getter = File;
        getter.get(dest, "file://./test-1.txt").unwrap();

        let mut df = File::open(dest).unwrap();
        let mut buf = Vec::new();
        df.read_to_end(&mut buf).unwrap();

        println!("{:?}", std::str::from_utf8(&buf).unwrap());
        assert_eq!(buf, "test".as_bytes());
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
