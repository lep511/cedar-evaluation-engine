use std::str::FromStr;

use cedar_policy::*;
use serde_json::{json, Value};

use crate::models::{UserRecord, USER_FIELDS};

pub fn load_policies() -> PolicySet {
    let policy_src =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/policies/marketing.cedar"));
    policy_src.parse().expect("failed to parse Cedar policies")
}

pub fn load_schema() -> Schema {
    let schema_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/policies/marketing.cedarschema"
    ));
    schema_src.parse().expect("failed to parse Cedar schema")
}

pub fn validate_policies(policies: &PolicySet, schema: &Schema) {
    let validator = Validator::new(schema.clone());
    let result = validator.validate(policies, ValidationMode::default());
    if result.validation_passed() {
        tracing::info!("All Cedar policies are valid against the schema");
    } else {
        for error in result.validation_errors() {
            tracing::error!("Cedar validation error: {error}");
        }
        panic!("Cedar policy validation failed");
    }
}

pub fn build_entities() -> Entities {
    let entities_json = json!([
        { "uid": { "type": "MarketingTeam", "id": "InternalTeam" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "MarketingTeam", "id": "ExternalPartner" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "UserDataField", "id": "userId" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "UserDataField", "id": "email" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "UserDataField", "id": "plansViewed" }, "attrs": {}, "parents": [] },
        { "uid": { "type": "UserDataField", "id": "registrationDate" }, "attrs": {}, "parents": [] }
    ]);

    Entities::from_json_str(&entities_json.to_string(), None)
        .expect("failed to parse Cedar entities")
}

/// Returns the list of field names that the given audience is authorized to access.
pub fn authorized_fields(
    audience: &str,
    policies: &PolicySet,
    entities: &Entities,
) -> Vec<String> {
    let authorizer = Authorizer::new();
    let mut allowed = Vec::new();

    for field in USER_FIELDS {
        let principal = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str("MarketingTeam").unwrap(),
            EntityId::from_str(audience).unwrap(),
        );
        let action = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str("Action").unwrap(),
            EntityId::from_str("access").unwrap(),
        );
        let resource = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str("UserDataField").unwrap(),
            EntityId::from_str(field).unwrap(),
        );

        let request =
            Request::new(principal, action, resource, Context::empty(), None).unwrap();

        let response = authorizer.is_authorized(&request, policies, entities);

        if matches!(response.decision(), Decision::Allow) {
            allowed.push(field.to_string());
        }
    }

    allowed
}

/// Filters a UserRecord to include only the authorized fields.
pub fn filter_user(user: &UserRecord, allowed_fields: &[String]) -> Value {
    let mut obj = serde_json::Map::new();

    for field in allowed_fields {
        match field.as_str() {
            "userId" => {
                obj.insert("userId".into(), json!(user.user_id));
            }
            "email" => {
                obj.insert("email".into(), json!(user.email));
            }
            "plansViewed" => {
                obj.insert("plansViewed".into(), json!(user.plans_viewed));
            }
            "registrationDate" => {
                obj.insert("registrationDate".into(), json!(user.registration_date));
            }
            _ => {}
        }
    }

    Value::Object(obj)
}
