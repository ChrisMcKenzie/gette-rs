use path_clean::PathClean;
use std::{
    env,
    path::{Path, PathBuf},
};

pub struct File;
impl crate::Detector for File {
    fn detect(&self, path: &str) -> Result<Option<String>, crate::Error> {
        if url::Url::parse(path).is_ok() {
            return Ok(None);
        }

        let path = absolute_path(path)?;

        Ok(Some(
            format!("file://{}", path.to_str().unwrap()).to_string(),
        ))
    }
}

fn absolute_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, crate::Error> {
    let path = path.as_ref();
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }
    .clean();

    Ok(abs)
}
