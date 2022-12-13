//! Example object upload/retrieval app, using Multipart.
//! Does not work with files smaller than 5MB, and is not tested.
use std::collections::HashMap;

use anyhow::Result;
use rusoto_core::{credential::EnvironmentProvider, ByteStream, HttpClient, Region, RusotoError};
use rusoto_s3::{
    CompleteMultipartUploadRequest, CreateBucketError, CreateBucketRequest,
    CreateMultipartUploadRequest, GetObjectRequest, S3Client, UploadPartRequest, S3, CompletedPart, CompletedMultipartUpload,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

const BUCKET: &str = "my-s3-bucket";
const OBJECT: &str = "my-dfu.elf";
const CHUNK_SIZE: usize = 128;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let region = Region::Custom {
        name: "local".to_owned(),
        endpoint: "http://127.0.0.1:9000".to_owned(),
    };

    let http_client = HttpClient::new()?;
    let credentials_provider = EnvironmentProvider::default();

    let s3_client = S3Client::new_with(http_client, credentials_provider, region);
    let buckets = s3_client.list_buckets().await;
    dbg!(buckets)?;

    match s3_client
        .create_bucket(CreateBucketRequest {
            bucket: BUCKET.to_owned(),
            ..Default::default()
        })
        .await
    {
        Ok(_) => {}
        Err(RusotoError::Service(CreateBucketError::BucketAlreadyOwnedByYou(_))) => {}
        Err(e) => Err(dbg!(e))?,
    }

    let upload = s3_client
        .create_multipart_upload(CreateMultipartUploadRequest {
            bucket: BUCKET.to_owned(),
            key: OBJECT.to_owned(),
            expected_bucket_owner: None, // TODO make sure we're the owner
            metadata: Some(HashMap::from_iter([
                ("version".to_owned(), "0.1.0".to_owned()),
                ("project_id".to_owned(), "1234".to_owned()),
            ])),
            ..Default::default()
        })
        .await?;
    dbg!(&upload);

    let upload_id = upload.upload_id.unwrap();
    let file = File::open(OBJECT).await?;
    let mut file = BufReader::new(file);

    let mut buf = [0u8; CHUNK_SIZE];

    let mut uploaded_parts = vec![];
    for part_number in 1.. {

        let body = match file.read(&mut buf).await? {
            0 => break,
            n => Vec::from(&buf[..n]),
        };

        dbg!(part_number, body.len());

        // Upload in parts of 128 bytes.
        let part_upload = s3_client
            .upload_part(UploadPartRequest {
                body: Some(ByteStream::from(body)),
                part_number,
                bucket: BUCKET.to_owned(),
                key: OBJECT.to_owned(),
                expected_bucket_owner: None, // TODO make sure we're the owner
                upload_id: upload_id.clone(),
                ..Default::default()
            })
            .await?;
        dbg!(&part_upload);
        uploaded_parts.push(CompletedPart {
            e_tag: part_upload.e_tag,
            part_number: Some(part_number)
        });
    }
    println!("Done!");

    s3_client
        .complete_multipart_upload(CompleteMultipartUploadRequest {
            bucket: BUCKET.to_owned(),
            key: OBJECT.to_owned(),
            upload_id: upload_id.clone(),
            expected_bucket_owner: None, // TODO make sure we're the owner
            multipart_upload: Some(CompletedMultipartUpload {
                parts: Some(uploaded_parts),
            }),
            ..Default::default()
        })
        .await?;

    println!("Completed");
    s3_client
        .get_object(GetObjectRequest {
            bucket: BUCKET.to_owned(),
            part_number: Some(1),
            key: OBJECT.to_owned(),
            expected_bucket_owner: None, // TODO make sure we're the owner
            ..Default::default()
        })
        .await?;

    Ok(())
}
