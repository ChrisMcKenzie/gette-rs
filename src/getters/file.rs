use crate::Error;
use std::{fs, path::Path};

pub struct Getter;

impl crate::Getter for Getter {
    fn get(&self, dest: &str, source: &str) -> Result<(), crate::Error> {
        self.get(dest, source)
    }

    fn copy(&self, _dest: &str, _source: &str) -> Result<(), crate::Error> {
        Ok(())
    }

    fn detect(&self, path: &str) -> bool {
        let u = url::Url::parse(path).unwrap();
        if u.scheme() == "file" {
            return true;
        }

        false
    }
}

impl Getter {
    #[cfg(target_family = "unix")]
    fn get(&self, dest: &str, source: &str) -> Result<(), crate::Error> {
        // validate source
        let source = Path::new(source);
        let dest = Path::new(dest);

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
        let source = Path::new(source);
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
        let source = "test.txt";
        let mut f = File::create(source).unwrap();
        f.write_all("test".as_bytes()).unwrap();

        let dest = "test2.txt";

        let getter = Getter;
        getter.get(dest, source).unwrap();

        let mut df = File::open(dest).unwrap();
        let mut buf = Vec::new();
        df.read_to_end(&mut buf).unwrap();

        println!("{:?}", std::str::from_utf8(&buf).unwrap());
        assert_eq!(buf, "test".as_bytes());
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
