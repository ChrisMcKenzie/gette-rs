use std::io::Write;

use async_trait::async_trait;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use futures::TryStreamExt;

use crate::Error;

pub type S3 = S3Getter<Client>;

impl Default for S3 {
    fn default() -> Self {
        Self { client: None }
    }
}

#[async_trait]
pub trait S3Client {
    async fn get_object(&self, bucket: &str, prefix: &str) -> Result<GetObjectOutput, Error>;
    async fn setup(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl Default for Client {
    fn default() -> Self {
        Self { client: None }
    }
}

pub struct Client {
    client: Option<aws_sdk_s3::Client>,
}

impl Client {
    fn set_client(&mut self, client: aws_sdk_s3::Client) {
        self.client = Some(client);
    }
}

#[async_trait]
impl S3Client for Client {
    async fn setup(&mut self) -> Result<(), Error> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);
        self.set_client(client);

        Ok(())
    }
    async fn get_object(&self, bucket: &str, prefix: &str) -> Result<GetObjectOutput, Error> {
        let client = self.client.as_ref().unwrap();
        client
            .get_object()
            .bucket(bucket)
            .key(prefix)
            .send()
            .await
            .map_err(|e| Error::Unknown(e.into_source().unwrap()))
    }
}

pub struct S3Getter<T>
where
    T: S3Client,
{
    client: Option<T>,
}

#[async_trait]
impl<T: S3Client + Sync + Send + Default> crate::Getter for S3Getter<T> {
    async fn set_client(&mut self) -> Result<(), Error> {
        if self.client.is_none() {
            let mut client = T::default();
            client.setup().await?;
            self.client = Some(client)
        }

        Ok(())
    }
    async fn get(&self, _dest: &str, source: &str) -> Result<(), Error> {
        let u = url::Url::parse(source)?;

        let client = self.client.as_ref().unwrap();

        let domain = u.domain().unwrap();
        let bucket = domain.split('.').next().unwrap();

        let path = u.path().strip_prefix('/').unwrap_or(u.path());

        let mut object = client.get_object(bucket, path).await?;

        let mut dest_file = std::fs::File::create(_dest)?;
        while let Some(chunk) = object
            .body
            .try_next()
            .await
            .map_err(|e| Error::Unknown(Box::new(e)))?
        {
            dest_file.write_all(&chunk)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use aws_sdk_s3::{
        operation::get_object::builders::GetObjectOutputBuilder,
        primitives::{ByteStream, SdkBody},
    };

    use std::fs;

    use super::*;
    use crate::Getter;

    struct MockS3Client {
        expected_bucket: String,
        expected_prefix: String,
        object: aws_sdk_s3::types::Object,
        content: String,
    }

    impl Default for MockS3Client {
        fn default() -> Self {
            Self {
                expected_bucket: "".to_string(),
                expected_prefix: "".to_string(),
                object: aws_sdk_s3::types::Object::builder().size(0).build(),
                content: "".to_string(),
            }
        }
    }

    #[async_trait]
    impl S3Client for MockS3Client {
        async fn get_object(&self, bucket: &str, prefix: &str) -> Result<GetObjectOutput, Error> {
            println!("bucket: {}, prefix: {}", bucket, prefix);
            if self.expected_bucket != bucket {
                return Err(Error::SourceNotFound);
            }

            if self.expected_prefix != prefix {
                return Err(Error::SourceNotFound);
            }

            Ok(GetObjectOutputBuilder::default()
                .body(ByteStream::from(SdkBody::from(self.content.as_str())))
                .content_length(self.object.size)
                .build())
        }
    }

    #[tokio::test]
    async fn it_should_get_files() {
        let client = MockS3Client {
            expected_bucket: "test".to_string(),
            expected_prefix: "test.txt".to_string(),
            object: aws_sdk_s3::types::Object::builder().size(10).build(),
            content: "test".to_string(),
        };

        let g: S3Getter<MockS3Client> = S3Getter {
            client: Some(client),
        };

        let dest = "test.txt";

        let res = g
            .get(dest, "https://test.s3.us-east-2.amazonaws.com/test.txt")
            .await;

        println!("{:#?}", res);
        assert!(res.is_ok());
        fs::remove_file(dest).unwrap();
    }
}
