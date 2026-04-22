use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub user_id: String,
    pub email: String,
    pub plans_viewed: Vec<String>,
    pub registration_date: String,
}

pub const USER_FIELDS: &[&str] = &["userId", "email", "plansViewed", "registrationDate"];

pub const AUDIENCES: &[&str] = &["InternalTeam", "ExternalPartner"];
