# Cedar Evaluation Engine

A Rust project that demonstrates how to use [Cedar](https://www.cedarpolicy.com/), an open-source policy language and authorization engine developed by AWS, to make fine-grained access control decisions.

## What is Cedar?

Cedar is a language for defining permissions as policies, which describe who should have access to what. Cedar answers the question:

> *"Given this set of policies, can this **principal** perform this **action** on this **resource**?"*

The authorization engine evaluates the request against the policy set and returns an **Allow** or **Deny** decision. By decoupling access control from application logic, Cedar makes permissions easier to write, understand, audit, and manage.

### Core Concepts

| Concept | Description |
|---|---|
| **Principal** | The entity making the request (e.g., `User::"alice"`) |
| **Action** | The operation being performed (e.g., `Action::"view"`) |
| **Resource** | The target of the action (e.g., `File::"93"`) |
| **Context** | Additional key-value data about the request (IP address, time, etc.) |
| **Policy** | A rule that `permit`s or `forbid`s a request |
| **PolicySet** | A collection of policies evaluated together |
| **Entity** | A principal, action, or resource with attributes and a parent hierarchy |
| **Schema** | Defines the valid entity types, actions, and their relationships |

### The Cedar Policy Language

Cedar policies use a human-readable syntax:

```cedar
// Allow alice to view file 93
permit(
    principal == User::"alice",
    action == Action::"view",
    resource == File::"93"
);

// Allow any admin group member to delete file 93
permit(
    principal in Group::"admin",
    action == Action::"delete",
    resource == File::"93"
);

// Explicitly deny bob from deleting file 93
forbid(
    principal == User::"bob",
    action == Action::"delete",
    resource == File::"93"
);
```

Key language features:

- **`permit`** and **`forbid`** — the two policy effects. If any `forbid` matches, the request is denied regardless of `permit` policies.
- **`==`** — exact match on an entity UID.
- **`in`** — checks if the principal/resource is a member of (or descendant of) an entity group.
- **`when` / `unless`** — optional condition clauses that reference entity attributes or context.
- **Templates** — policies with placeholder slots (`?principal`, `?resource`) that can be instantiated at runtime.

### Entity Hierarchy

Entities can have parent relationships, forming a hierarchy. For example, if `User::"alice"` is a member of `Group::"admin"`, a policy that permits `principal in Group::"admin"` will match alice's requests.

Entities are defined in JSON:

```json
[
    {
        "uid": { "type": "User", "id": "alice" },
        "attrs": { "age": 25 },
        "parents": [{ "type": "Group", "id": "admin" }]
    },
    {
        "uid": { "type": "Group", "id": "admin" },
        "attrs": {},
        "parents": []
    }
]
```

### Schema Validation

Cedar provides a `Validator` that checks policies against a schema to catch errors at authoring time rather than at runtime:

```cedar
entity User in [Group] { "age": Long };
entity Group;
entity File;
action "view" appliesTo { principal: User, resource: File };
action "delete" appliesTo { principal: [User, Group], resource: File };
```

### Authorization Flow

```
                ┌──────────────┐
                │  Application │
                └──────┬───────┘
                       │ Request(principal, action, resource, context)
                       v
              ┌─────────────────┐
              │   Authorizer    │
              │ is_authorized() │
              └────────┬────────┘
                       │
            ┌──────────┼──────────┐
            v          v          v
        PolicySet   Entities   Context
            │          │          │
            └──────────┼──────────┘
                       │
                       v
               Decision: Allow / Deny
               + Diagnostics (reasons, errors)
```

## Key API Types (`cedar-policy` crate)

| Type | Purpose |
|---|---|
| `Authorizer` | Evaluates authorization requests. Created with `Authorizer::new()`. |
| `Request` | Holds principal, action, resource, and context for an authorization query. |
| `PolicySet` | A collection of `Policy` and `Template` objects. Parsed from Cedar text or JSON. |
| `Entities` | An entity store with hierarchy. Loaded from JSON or constructed programmatically. |
| `Context` | Key-value map of request context. Supports `Context::empty()` or JSON construction. |
| `EntityUid` | Unique identifier for an entity (e.g., `User::"alice"`). Parseable from strings. |
| `Response` | Contains the `Decision` and `Diagnostics` (which policies matched, any errors). |
| `Decision` | Enum: `Allow` or `Deny`. |
| `Schema` | Defines the type system. Parsed from Cedar schema syntax or JSON. |
| `Validator` | Validates a `PolicySet` against a `Schema`. |
| `Entity` | Represents a single entity with UID, attributes, and parent references. |

## Project Structure

```
cedar-evaluation-engine/
├── Cargo.toml          # Project manifest — depends on cedar-policy 4.9.x
├── README.md           # This file
├── ABOUT_CEDAR.md      # In-depth article about Cedar, cedar-local-agent, and avp-local-agent
├── version.txt         # Cedar release notes for version 4.9.0
└── src/
    └── main.rs         # Example: policies, entities, authorization, and schema validation
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2021 or later)

## Getting Started

Clone the repository and run the example:

```bash
cargo run
```

### Expected Output

```
Cedar Policy Evaluation Engine
==================================================

Alice views file 93
  Decision: ALLOW
  Determined by policy: policy0

Alice deletes file 93 (admin)
  Decision: ALLOW
  Determined by policy: policy1

Bob views file 93 (no policy)
  Decision: DENY

Bob deletes file 93 (forbidden)
  Decision: DENY
  Determined by policy: policy2

==================================================
Schema Validation
==================================================
All policies are valid against the schema.
```

### What the Example Demonstrates

1. **Policy parsing** — three Cedar policies (`permit` and `forbid`) parsed from a string into a `PolicySet`.
2. **Entity hierarchy** — alice belongs to `Group::"admin"`, so she inherits group-level permissions. Bob has no group membership.
3. **Authorization evaluation** — four requests tested against the policy set:
   - Alice can **view** (direct permit) and **delete** (admin group permit).
   - Bob is **denied view** (no matching policy, default deny) and **denied delete** (explicit forbid).
4. **Schema validation** — all policies are validated against a Cedar schema to ensure correctness.

## Dependencies

| Crate | Version | Description |
|---|---|---|
| [cedar-policy](https://crates.io/crates/cedar-policy) | 4.9.x | Cedar policy language and authorization engine |

## Resources

- [Cedar Policy Website](https://www.cedarpolicy.com/)
- [Cedar Documentation](https://docs.cedarpolicy.com/)
- [cedar-policy crate on docs.rs](https://docs.rs/cedar-policy/latest/cedar_policy/)
- [Cedar GitHub Repository](https://github.com/cedar-policy/cedar)
- [Cedar Examples Repository](https://github.com/cedar-policy/cedar-examples)
- [Cedar Playground](https://www.cedarpolicy.com/en/playground)
