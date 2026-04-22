use cedar_policy::*;
use std::str::FromStr;

fn main() {
    // --- Define policies ---
    let policies: PolicySet = r#"
        permit(
            principal == User::"alice",
            action == Action::"view",
            resource == File::"93"
        );

        permit(
            principal in Group::"admin",
            action == Action::"delete",
            resource == File::"93"
        );

        forbid(
            principal == User::"bob",
            action == Action::"delete",
            resource == File::"93"
        );
    "#
    .parse()
    .expect("failed to parse policies");

    // --- Define entities with hierarchy ---
    let entities = Entities::from_json_str(
        r#"[
            {
                "uid": { "type": "User", "id": "alice" },
                "attrs": { "age": 25 },
                "parents": [{ "type": "Group", "id": "admin" }]
            },
            {
                "uid": { "type": "User", "id": "bob" },
                "attrs": { "age": 30 },
                "parents": []
            },
            {
                "uid": { "type": "Group", "id": "admin" },
                "attrs": {},
                "parents": []
            },
            {
                "uid": { "type": "File", "id": "93" },
                "attrs": {},
                "parents": []
            }
        ]"#,
        None,
    )
    .expect("failed to parse entities");

    let authorizer = Authorizer::new();

    // --- Test cases ---
    let test_cases = vec![
        ("User", "alice", "Action", "view", "File", "93", "Alice views file 93"),
        ("User", "alice", "Action", "delete", "File", "93", "Alice deletes file 93 (admin)"),
        ("User", "bob", "Action", "view", "File", "93", "Bob views file 93 (no policy)"),
        ("User", "bob", "Action", "delete", "File", "93", "Bob deletes file 93 (forbidden)"),
    ];

    println!("Cedar Policy Evaluation Engine");
    println!("{}", "=".repeat(50));

    for (p_type, p_id, a_type, a_id, r_type, r_id, description) in &test_cases {
        let principal = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str(p_type).unwrap(),
            EntityId::from_str(p_id).unwrap(),
        );
        let action = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str(a_type).unwrap(),
            EntityId::from_str(a_id).unwrap(),
        );
        let resource = EntityUid::from_type_name_and_id(
            EntityTypeName::from_str(r_type).unwrap(),
            EntityId::from_str(r_id).unwrap(),
        );

        let request =
            Request::new(principal, action, resource, Context::empty(), None).unwrap();

        let response = authorizer.is_authorized(&request, &policies, &entities);

        let decision = match response.decision() {
            Decision::Allow => "ALLOW",
            Decision::Deny => "DENY",
        };

        println!("\n{description}");
        println!("  Decision: {decision}");

        for reason in response.diagnostics().reason() {
            println!("  Determined by policy: {reason}");
        }
        for error in response.diagnostics().errors() {
            println!("  Error: {error}");
        }
    }

    // --- Schema validation example ---
    println!("\n{}", "=".repeat(50));
    println!("Schema Validation");
    println!("{}", "=".repeat(50));

    let schema: Schema = r#"
        entity User in [Group] { "age": Long };
        entity Group;
        entity File;
        action "view" appliesTo { principal: User, resource: File };
        action "delete" appliesTo { principal: [User, Group], resource: File };
    "#
    .parse()
    .expect("failed to parse schema");

    let validator = Validator::new(schema);
    let result = validator.validate(&policies, ValidationMode::default());

    if result.validation_passed() {
        println!("All policies are valid against the schema.");
    } else {
        for error in result.validation_errors() {
            println!("Validation error: {error}");
        }
        for warning in result.validation_warnings() {
            println!("Validation warning: {warning}");
        }
    }
}
