use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use serde_json::Value;

/// Writes the filtered user data as JSON to an S3 object keyed by audience.
pub async fn write_audience_data(
    client: &Client,
    bucket: &str,
    audience: &str,
    data: &[Value],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = format!("{}/users.json", audience);
    let body = serde_json::to_string_pretty(data)?;

    client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .content_type("application/json")
        .body(ByteStream::from(body.into_bytes()))
        .send()
        .await?;

    Ok(())
}
