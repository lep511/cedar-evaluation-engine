# Marketing Data Pipeline with Cedar Authorization

## Overview

This example implements a serverless marketing data pipeline that reads user records from a DynamoDB table and writes filtered data to an S3 bucket. **Cedar policies** enforce field-level access control, ensuring each audience only receives the data they are authorized to see.

Two audiences consume the data:

| Audience | Accessible Fields | Rationale |
|---|---|---|
| **Internal Marketing Team** | `userId`, `email`, `plansViewed`, `registrationDate` | Full access for internal analytics |
| **External Marketing Partner** | `plansViewed`, `registrationDate` | PII fields (`userId`, `email`) are excluded |

## Architecture

```
┌──────────────────────┐
│   DynamoDB Table     │
│  userId │ email      │
│  plansViewed         │
│  registrationDate    │
└─────────┬────────────┘
          │
          ▼
┌──────────────────────┐       ┌─────────────┐     ┌──────────────────────┐
│   Lambda Function    │──────▶│  S3 Bucket  │────▶│ Internal Marketing   │
│                      │       │  (shared)   │     │ Team                 │
│  1. Scan DynamoDB    │       └─────────────┘     ├──────────────────────┤
│  2. Evaluate Cedar   │              │            │ External Marketing   │
│  3. Filter fields    │              └───────────▶│ Partner              │
│  4. Write to S3      │                           └──────────────────────┘
└──────────────────────┘
```

The Lambda function reads **all** users from the DynamoDB table. For each audience, it evaluates Cedar authorization policies to determine which fields are allowed, filters the data accordingly, and writes the result as a JSON file to S3.

## How Cedar Authorization Works

### Entity Model

Each data field is modeled as a Cedar **resource** of type `UserDataField`. Each audience is a **principal** of type `MarketingTeam`. A single action `access` represents the ability to read a field.

```
Principals:    MarketingTeam::"InternalTeam"
               MarketingTeam::"ExternalPartner"

Resources:     UserDataField::"userId"
               UserDataField::"email"
               UserDataField::"plansViewed"
               UserDataField::"registrationDate"

Action:        Action::"access"
```

### Policies

```cedar
// Internal Marketing Team can access all user data fields
permit(
    principal == MarketingTeam::"InternalTeam",
    action == Action::"access",
    resource
);

// External Marketing Partner can access plansViewed
permit(
    principal == MarketingTeam::"ExternalPartner",
    action == Action::"access",
    resource == UserDataField::"plansViewed"
);

// External Marketing Partner can access registrationDate
permit(
    principal == MarketingTeam::"ExternalPartner",
    action == Action::"access",
    resource == UserDataField::"registrationDate"
);
```

The **Internal Team** policy uses an unrestricted `resource` clause, granting access to any `UserDataField`. The **External Partner** requires an explicit `permit` per field — adding a new field to their view requires a deliberate policy change.

### Authorization Flow

For each audience, the Lambda iterates over every field name and asks Cedar:

> *Can `MarketingTeam::{audience}` perform `Action::"access"` on `UserDataField::{field}`?*

Only fields that receive an `ALLOW` decision are included in the output.

## File Structure

```
examples/marketing-pipeline/
├── Cargo.toml                     # Dependencies and build configuration
├── MARKETING-SAMPLE.md            # This file
├── policies/
│   ├── marketing.cedar            # Cedar policy definitions
│   └── marketing.cedarschema      # Cedar entity and action schema
└── src/
    ├── main.rs                    # Lambda entry point and handler
    ├── cedar_auth.rs              # Cedar policy loading and authorization logic
    ├── dynamo.rs                  # DynamoDB scan with pagination
    ├── s3_writer.rs               # S3 JSON upload per audience
    └── models.rs                  # UserRecord struct and constants
```

## Prerequisites

- **Rust** toolchain (edition 2021) — install via [rustup](https://rustup.rs/)
- **AWS account** with:
  - A DynamoDB table containing user records with attributes: `userId` (S), `email` (S), `plansViewed` (L of S), `registrationDate` (S)
  - An S3 bucket for output
- **Lambda execution role** with permissions:
  - `dynamodb:Scan` on the user table
  - `s3:PutObject` on the output bucket

## Environment Variables

| Variable | Description | Example |
|---|---|---|
| `DYNAMODB_TABLE` | DynamoDB table name with user records | `marketing-users` |
| `S3_BUCKET` | S3 bucket name for filtered output | `marketing-data-output` |

## Building and Deploying

### Build for Lambda (Amazon Linux 2)

```bash
# Install the musl target for static linking
rustup target add x86_64-unknown-linux-musl

# Build the release binary
cargo build --release --target x86_64-unknown-linux-musl

# Package for Lambda deployment
cp target/x86_64-unknown-linux-musl/release/marketing-pipeline ./bootstrap
zip lambda.zip bootstrap
```

Deploy `lambda.zip` to AWS Lambda with the **provided.al2023** runtime.

### Build with cargo-lambda (alternative)

```bash
cargo install cargo-lambda
cargo lambda build --release
cargo lambda deploy --iam-role arn:aws:iam::ACCOUNT_ID:role/ROLE_NAME
```

## Sample Output

### InternalTeam/users.json

All fields are included:

```json
[
  {
    "userId": "u-001",
    "email": "alice@example.com",
    "plansViewed": ["Basic", "Premium"],
    "registrationDate": "2025-01-15"
  },
  {
    "userId": "u-002",
    "email": "bob@example.com",
    "plansViewed": ["Enterprise"],
    "registrationDate": "2025-03-22"
  }
]
```

### ExternalPartner/users.json

PII fields (`userId`, `email`) are excluded by Cedar policy:

```json
[
  {
    "plansViewed": ["Basic", "Premium"],
    "registrationDate": "2025-01-15"
  },
  {
    "plansViewed": ["Enterprise"],
    "registrationDate": "2025-03-22"
  }
]
```

## Extending the Example

- **Add a new audience**: Create a `MarketingTeam` entity and write explicit `permit` policies for the fields they should access.
- **Add a new data field**: Add a `UserDataField` resource. The Internal Team gets access automatically (unrestricted `resource` clause). External partners require a new explicit `permit` policy.
- **Restrict a field**: Add a `forbid` policy. For example, to block email access for all non-internal teams:
  ```cedar
  forbid(
      principal,
      action == Action::"access",
      resource == UserDataField::"email"
  ) unless {
      principal == MarketingTeam::"InternalTeam"
  };
  ```
