pub struct S3;

impl crate::Detector for S3 {
    fn detect(&self, path: &str) -> Result<Option<String>, crate::Error> {
        if path.contains("amazonaws.com/") {
            return self.detect_http(path);
        }

        Ok(None)
    }
}

impl S3 {
    fn detect_http(&self, path: &str) -> Result<Option<String>, crate::Error> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 2 {
            return Err(crate::Error::InvalidUrl(
                path.to_string(),
                "not a valid s3 url".to_string(),
            ));
        }

        let host: Vec<&str> = parts[0].split('.').collect();
        match host.len() {
            3 => self.region_path_style(host[0], parts[1..].to_vec()),
            4 => self.vhost_path_style(host[1], host[0], parts[1..].to_vec()),
            5 if host[1] == "s3" => {
                self.new_vhost_path_style(host[2], host[0], parts[1..].to_vec())
            }
            _ => Err(crate::Error::InvalidUrl(
                path.to_string(),
                "not a valid s3 url".to_string(),
            )),
        }
    }

    fn region_path_style(
        &self,
        region: &str,
        parts: Vec<&str>,
    ) -> Result<Option<String>, crate::Error> {
        let url_string = format!("https://{}.amazonaws.com/{}", region, parts.join("/"));
        let url_parsed = url::Url::parse(url_string.as_str())?;
        Ok(Some(format!("s3+{}", url_parsed)))
    }

    fn vhost_path_style(
        &self,
        region: &str,
        bucket: &str,
        parts: Vec<&str>,
    ) -> Result<Option<String>, crate::Error> {
        let url_string = format!(
            "https://{}.amazonaws.com/{}/{}",
            region,
            bucket,
            parts.join("/")
        );
        let url_parsed = url::Url::parse(url_string.as_str())?;
        Ok(Some(format!("s3+{}", url_parsed)))
    }

    fn new_vhost_path_style(
        &self,
        region: &str,
        bucket: &str,
        parts: Vec<&str>,
    ) -> Result<Option<String>, crate::Error> {
        let url_string = format!(
            "https://s3.{}.amazonaws.com/{}/{}",
            region,
            bucket,
            parts.join("/")
        );
        let url_parsed = url::Url::parse(url_string.as_str())?;
        Ok(Some(format!("s3+{}", url_parsed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Detector;

    #[test]
    fn it_should_decode_all_valid_variants_of_s3_urls() {
        let d = S3;
        let tests = vec![
            (
                "test.us-east-2.amazonaws.com/test.txt",
                "s3+https://us-east-2.amazonaws.com/test/test.txt",
            ),
            (
                "test.s3.us-east-2.amazonaws.com/test.txt",
                "s3+https://s3.us-east-2.amazonaws.com/test/test.txt",
            ),
            (
                "us-east-2.amazonaws.com/test/test.txt",
                "s3+https://us-east-2.amazonaws.com/test/test.txt",
            ),
        ];

        for test in tests {
            let res = d.detect(test.0).unwrap();
            assert!(res.is_some());
            assert_eq!(res, Some(test.1.to_string()));
        }
    }

    #[test]
    fn it_should_fail_on_invalid_s3_urls() {
        let d = S3;
        let tests = vec![
            "wrong.test.us-east-2.amazonaws.com/test.txt",
            "amazonaws.com/test.txt",
        ];

        for test in tests {
            let res = d.detect(test);
            println!("test: ({}) => {:#?}", test, res);
            assert!(res.is_err());
        }
    }
}
