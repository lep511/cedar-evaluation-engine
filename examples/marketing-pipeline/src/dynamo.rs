use aws_sdk_dynamodb::Client;

use crate::models::UserRecord;

/// Scans all user records from the DynamoDB table, handling pagination.
pub async fn scan_all_users(
    client: &Client,
    table_name: &str,
) -> Result<Vec<UserRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let mut users = Vec::new();
    let mut exclusive_start_key = None;

    loop {
        let mut request = client.scan().table_name(table_name);

        if let Some(key) = exclusive_start_key.take() {
            request = request.set_exclusive_start_key(Some(key));
        }

        let response = request.send().await?;

        if let Some(items) = response.items {
            for item in items {
                let user_id = item
                    .get("userId")
                    .and_then(|v| v.as_s().ok())
                    .unwrap_or(&String::new())
                    .clone();

                let email = item
                    .get("email")
                    .and_then(|v| v.as_s().ok())
                    .unwrap_or(&String::new())
                    .clone();

                let plans_viewed = item
                    .get("plansViewed")
                    .and_then(|v| v.as_l().ok())
                    .map(|list| {
                        list.iter()
                            .filter_map(|v| v.as_s().ok().cloned())
                            .collect()
                    })
                    .unwrap_or_default();

                let registration_date = item
                    .get("registrationDate")
                    .and_then(|v| v.as_s().ok())
                    .unwrap_or(&String::new())
                    .clone();

                users.push(UserRecord {
                    user_id,
                    email,
                    plans_viewed,
                    registration_date,
                });
            }
        }

        match response.last_evaluated_key {
            Some(key) if !key.is_empty() => {
                exclusive_start_key = Some(key);
            }
            _ => break,
        }
    }

    Ok(users)
}
