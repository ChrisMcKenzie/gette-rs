use crate::Error;
use std::{env, fs, path::Path};
use std::path::PathBuf;
use url::{Position, Url};

use path_clean::{PathClean};

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
        let source = absolute_path(Path::new(&u[Position::BeforeUsername..]))?;
        let dest = absolute_path(Path::new(dest))?;

        let source = source.as_path();
        let dest = dest.as_path();

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
        let source = absolute_path(Path::new(&u[Position::BeforeUsername..]))?;
        let dest = absolute_path(Path::new(dest))?;

        let source = source.as_path();
        let dest = dest.as_path();

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
            std::os::windows::fs::symlink_dir(source, dest)?;
        } else {
            std::os::windows::fs::symlink_file(source, dest)?;
        }

        Ok(())
    }
}

fn absolute_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, crate::Error> {
    let path = path.as_ref();
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }.clean();

    Ok(abs)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{ Write},
    };
    use std::io::Read;

    use super::*;

    #[test]
    fn test_get_file_from_tmp() {
        let source = "./test-1.txt";
        if !Path::new(source).exists() {
            let mut f = File::create(source).unwrap();
            f.write_all("test".as_bytes()).unwrap();
        }

        let dest = "test-2.txt";

        let getter = File;
        getter.get(dest, "file://./test-1.txt").unwrap();

        assert!(Path::new(dest).exists());

        let mut df = File::open(dest).unwrap();
        let mut buf = Vec::new();
        df.read_to_end(&mut buf).unwrap();

        println!("file contents = {:?}", std::str::from_utf8(&buf).unwrap());
        assert_eq!(buf, "test".as_bytes());
        fs::remove_file(source).unwrap();
        fs::remove_file(dest).unwrap();
    }
}
