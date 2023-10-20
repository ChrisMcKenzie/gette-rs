use std::io::Write;

use async_trait::async_trait;
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use futures::{executor::block_on, TryStreamExt};

use crate::Error;

pub type S3 = S3Getter<Client>;

#[async_trait]
pub trait S3Client {
    async fn get_object(&self, bucket: &str, prefix: &str) -> Result<GetObjectOutput, Error>;
}

pub struct Client {
    client: aws_sdk_s3::Client,
}

impl Client {
    pub fn new(client: aws_sdk_s3::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl S3Client for Client {
    async fn get_object(&self, bucket: &str, prefix: &str) -> Result<GetObjectOutput, Error> {
        self.client
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
    client: T,
}

impl Default for S3Getter<Client> {
    fn default() -> Self {
        let config = block_on(aws_config::from_env().load());

        Self {
            client: Client::new(aws_sdk_s3::Client::new(&config)),
        }
    }
}

#[async_trait]
impl<T: S3Client + Sync + Send> crate::Getter for S3Getter<T> {
    async fn get(&self, _dest: &str, source: &str) -> Result<(), Error> {
        let u = url::Url::parse(source)?;

        let domain = u.domain().unwrap();
        let bucket = domain.split('.').next().unwrap();

        let path = u.path().strip_prefix('/').unwrap_or(u.path());

        let mut object = self.client.get_object(bucket, path).await?;

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

        let g = S3Getter { client };

        let dest = "test.txt";

        let res = g
            .get(dest, "https://test.s3.us-east-2.amazonaws.com/test.txt")
            .await;

        println!("{:#?}", res);
        assert!(res.is_ok());
        fs::remove_file(dest).unwrap();
    }
}
