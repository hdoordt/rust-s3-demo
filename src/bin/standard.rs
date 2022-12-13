//! Example object upload/retrieval app

use std::collections::HashMap;

use anyhow::Result;
use rusoto_core::{credential::EnvironmentProvider, ByteStream, HttpClient, Region, RusotoError};
use rusoto_s3::{
    CreateBucketError,
    CreateBucketRequest, GetObjectRequest, PutObjectRequest,
    S3Client, S3,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

const BUCKET: &str = "my-s3-bucket";
const OBJECT: &str = "my-dfu.elf";

#[tokio::main]
async fn main() -> Result<()> {
    // Read contents from .env file into env vars
    dotenv::dotenv().ok();

    // Setup connection
    let region = Region::Custom {
        name: "local".to_owned(),
        endpoint: "http://127.0.0.1:9000".to_owned(),
    };

    let http_client = HttpClient::new()?;
    // Reads credentials from env
    let credentials_provider = EnvironmentProvider::default();

    // The s3 client. Don't forget to import the [S3] trait, 
    // which enables the functionality used below.
    let s3_client = S3Client::new_with(http_client, credentials_provider, region);

    // List buckets
    let buckets = s3_client.list_buckets().await;
    dbg!(buckets)?;

    // Create a bucket
    match s3_client
        .create_bucket(CreateBucketRequest {
            bucket: BUCKET.to_owned(),
            ..Default::default()
        })
        .await
    {
        Ok(_) => {} // pass, bucket created
        Err(RusotoError::Service(CreateBucketError::BucketAlreadyOwnedByYou(_))) => {} // pass, bucket already exists
        Err(e) => Err(dbg!(e))?,
    }

    // Read file into Vec
    let file = File::open(OBJECT).await?;
    let mut file = BufReader::new(file);
    let mut body = vec![];
    file.read_to_end(&mut body).await?;

    // Upload it
    s3_client
        .put_object(PutObjectRequest {
            bucket: BUCKET.to_owned(),
            key: OBJECT.to_owned(),
            expected_bucket_owner: None, // TODO make sure we're the owner
            metadata: Some(HashMap::from_iter([
                ("version".to_owned(), "0.1.0".to_owned()),
                ("project_id".to_owned(), "1234".to_owned()),
            ])),
            body: Some(ByteStream::from(body)),
            ..Default::default()
        })
        .await?;

    println!("Done uploading");

    // Fetch first 128 bytes of just uploaded object
    let object = s3_client
        .get_object(GetObjectRequest {
            bucket: BUCKET.to_owned(),
            range: Some("bytes=0-127".to_owned()),
            key: OBJECT.to_owned(),
            expected_bucket_owner: None, // TODO make sure we're the owner
            ..Default::default()
        })
        .await?;

    // Compare response body with first 128 bytes of file body
    dbg!(&object);
    let mut body = object.body.unwrap().into_async_read();
    let mut file = File::open(OBJECT).await?;

    let mut body_buf = vec![];
    let mut file_buf = vec![0u8; 128];
    let body = body.read_to_end(&mut body_buf).await?;
    let file = file.read(&mut file_buf[..128]).await?;
    

    assert_eq!(&body_buf[..body], &file_buf[..file]);
    println!("Success!");
    Ok(())
}
