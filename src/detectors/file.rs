pub struct File;
impl crate::Detector for File {
    fn detect(&self, path: &str) -> Result<Option<String>, crate::Error> {
        Ok(Some(format!("file://{}", path).to_string()))
    }
}
