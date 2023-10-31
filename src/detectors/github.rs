pub struct Github;
impl crate::Detector for Github {
    fn detect(&self, path: &str) -> Result<Option<String>, crate::Error> {
        if path.strip_prefix("github.com/").is_some() {
            return self.detect_http(path);
        }

        Ok(None)
    }
}

impl Github {
    fn detect_http(&self, path: &str) -> Result<Option<String>, crate::Error> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 3 {
            return Err(crate::Error::InvalidUrl(
                path.to_string(),
                "github.com urls should have the following format github.com/:username/:repo"
                    .to_string(),
            ));
        }

        let url_string = format!("https://{}", parts[..3].join("/"));
        let mut url_parsed = url::Url::parse(url_string.as_str())?;

        if url_parsed.path().strip_suffix(".git").is_none() {
            url_parsed.set_path(format!("{}.git", url_parsed.path()).as_str())
        }

        if parts.len() > 3 {
            url_parsed.set_path(format!("{}//{}", url_parsed.path(), parts[3..].join("/")).as_str())
        }

        Ok(Some(url_parsed.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::Detector;

    use super::*;

    #[test]
    fn it_detects_url_without_proto() {
        let d = Github;
        let res = d.detect("github.com/chrismckenzie/gette-rs").unwrap();
        assert!(res.is_some());
        assert_eq!(
            res,
            Some("https://github.com/chrismckenzie/gette-rs.git".to_string()),
        )
    }

    #[test]
    fn it_detects_url_without_proto_and_specific_path() {
        let d = Github;
        let res = d
            .detect("github.com/chrismckenzie/gette-rs/src/lib.rs")
            .unwrap();
        assert!(res.is_some());
        assert_eq!(
            res,
            Some("https://github.com/chrismckenzie/gette-rs.git//src/lib.rs".to_string()),
        )
    }

    #[test]
    fn it_detects_url_with_proto() {
        let d = Github;
        let res = d
            .detect("git+https://github.com/chrismckenzie/gette-rs/src/lib.rs")
            .unwrap();
        assert!(res.is_none());
    }
}
