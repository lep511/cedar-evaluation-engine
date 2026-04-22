mod cedar_auth;
mod dynamo;
mod models;
mod s3_writer;

use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};
use std::env;

use crate::models::AUDIENCES;

async fn handler(_event: LambdaEvent<Value>) -> Result<Value, Error> {
    let table_name = env::var("DYNAMODB_TABLE")
        .expect("DYNAMODB_TABLE environment variable is required");
    let bucket_name =
        env::var("S3_BUCKET").expect("S3_BUCKET environment variable is required");

    // Initialize AWS SDK clients
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let dynamo_client = aws_sdk_dynamodb::Client::new(&config);
    let s3_client = aws_sdk_s3::Client::new(&config);

    // Step 1: Scan all users from DynamoDB
    let users = dynamo::scan_all_users(&dynamo_client, &table_name).await?;
    tracing::info!("Scanned {} users from DynamoDB", users.len());

    // Step 2: Load Cedar policies, validate against schema, and build entities
    let policies = cedar_auth::load_policies();
    let schema = cedar_auth::load_schema();
    cedar_auth::validate_policies(&policies, &schema);
    let entities = cedar_auth::build_entities();

    // Step 3: For each audience, authorize fields and write filtered data to S3
    for audience in AUDIENCES {
        let allowed_fields = cedar_auth::authorized_fields(audience, &policies, &entities);
        tracing::info!(
            "Audience {}: allowed fields = {:?}",
            audience,
            allowed_fields
        );

        let filtered: Vec<Value> = users
            .iter()
            .map(|u| cedar_auth::filter_user(u, &allowed_fields))
            .collect();

        s3_writer::write_audience_data(&s3_client, &bucket_name, audience, &filtered)
            .await?;
        tracing::info!("Wrote {}/users.json to S3", audience);
    }

    Ok(json!({
        "statusCode": 200,
        "body": format!(
            "Processed {} users for {} audiences",
            users.len(),
            AUDIENCES.len()
        )
    }))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    lambda_runtime::run(service_fn(handler)).await
}
