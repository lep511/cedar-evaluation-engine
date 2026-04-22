# Cedar Evaluation Engine

Developers can use [Cedar, an open source policy language and evaluation engine](https://www.cedarpolicy.com/), to decouple access control from application logic by expressing fine-grained permissions as easy-to-understand policies. This blog post explains how developers using the Cedar SDK can use two new open source Rust crates, `cedar-local-agent` and `avp-local-agent`, to reduce their development burden and ease policy management tasks.

Developers use Cedar to answer the question, “Is this request authorized?” In Cedar terms, this question can be restated: “Given this set of policies, can this principal perform this action on this resource?” The Cedar authorization engine evaluates the request, resulting in an `Allow` or `Deny` decision. The application enforces this decision by allowing or denying the requested access. Figure 1 shows the high-level architecture of an application using Cedar, with the Cedar SDK intermediating access to the application. The application uses the SDK to create an authorization engine that makes authorization decisions based on policy sets and entities that the developer has provided to it.

![Cedar application architecture diagram](https://d2908q01vomqb2.cloudfront.net/ca3512f4dfa95a03169c5a670a4c91a19b3077b4/2023/12/13/Cedar-application-architecture.png)

Figure 1: The high-level architecture of an application using Cedar.

The authorization engine makes these decisions by evaluating policies. The following is an example Cedar policy:

```
permit (
  principal in Role::"boxManagers",
  action == Action::"update",
  resource == Box::"1"
);
```

This policy says that any principal that is a member of `Role::"boxManagers"` is allowed to take the action, `Action::"update"`, on the resource, `Box:: "1 "`. Determining whether a principal is allowed to take a specific action on a specific resource requires a developer to properly format the request and pass it to the Cedar SDK’s `is_authorized()` function, along with parameters specifying the policy sets to use in evaluating the request.

In addition to handling policy and schema management, developers using the Cedar SDK must decide how to store information about the principals and resources (jointly called entities) that are used in the authorization request. For example, if a policy allows all owners of a resource to take any action on it, the authorizer needs to know not only about the principal and resources, but also which principals own which resources. Another requirement for a fully functioning authorizer is logging. Most applications log requests to support monitoring, auditing, and debugging.

This post covers two new open source Rust crates that reduce the development load imposed by managing policies and entities as well as the effort involved in securely logging access decisions. The first, `cedar-local-agent`, builds scaffolding around the Cedar SDK to support both of these tasks, including the means to manage policies and entities on the file system. The second, `avp-local-agent`, extends these capabilities to support the use of [Amazon Verified Permissions](https://aws.amazon.com/verified-permissions/) as a central policy store.

## The cedar-local-agent

For simple applications, policy and entity data can be held in static structures maintained in the code. But for many applications, managing policies and entities independently from the application logic is important because then the policies can be managed separately from the code. In these applications, the developer needs to provide a control plane to manage policies, validate and test those policies, and potentially extend the policy schema. Entities might be stored in the application database and need to be available to the authorizer in a format it can use.

A policy store is a container for policies and policy templates. Each policy store contains a schema that is used to validate policies added to the policy store. The easiest strategy for implementing a policy store uses the local disk. This can be as simple as a versioned repository. An application still has to read the files containing policies, policy templates, and schema, cache them to avoid disk operations for every policy decision, and update the cache whenever a policy, templates, or schema file changes.

The `cedar-local-agent` is an open source Rust crate that provides a configurable cache for Cedar policies and entities. The crate provides a `simple::Authorizer` type that reduces the developer burden of using the Cedar SDK in several ways:

- First, `cedar-local-agent` includes a configurable policy cache that keeps policies in memory for faster policy evaluation.
- Second, `cedar-local-agent` provides a configurable entity provider that uses the [Cedar policy schema](https://www.cedarpolicy.com/en/tutorial/schema) to generate action groups.
- Last, `cedar-local-agent` builds in logging for important events, filters them, formats them, and writes them to disk.

Constructing an `Authorizer` requires that the developer supply policy and entity providers. Developers can implement their own providers to match their specific storage requirements, but the repository includes sample providers for storing files and entities on disk. Figure 2 shows an application architecture utilizing the `cedar-local-agent` with file-system-based policy and entity storage. The `cedar-local-agent` makes access decisions for the application based on the policy and entity data stored in them as well as formatting and writing logs.

![Cedar local agent architecture diagram](https://d2908q01vomqb2.cloudfront.net/ca3512f4dfa95a03169c5a670a4c91a19b3077b4/2023/12/13/cedar-local-agent-diagram.png)

Figure 2: Architecture of application using cedar-local-agent.

As mentioned previously, `Authorizer` also implements logging to facilitate monitoring, auditing, and debugging. Authorization events are formatted using the [Open Cybersecurity Schema Framework (OCSF)](https://schema.ocsf.io/). Authorization events are emitted in the form of [tracing events](https://docs.rs/tracing/latest/tracing/struct.Event.html), making it easier to integrate with standard Rust tracing implementations. Developers can configure the logger to filter which items are included in the logs. For examples of how to set up authorization logging, see the [usage examples](https://github.com/cedar-policy/cedar-local-agent/tree/main/examples/tracing/authorization_log).

## Using the file system Provider in cedar-local-agent

Developers can write policy and entity providers that meet their needs. For example, some applications might store policy information in the application database. The `cedar-local-agent` comes with example providers that use the local file system as a repository for policies and entity data.

This example shows creating a policy provider for policies stored on the local file system:

```
let policy_set_provider = PolicySetProvider::new(
    policy_set_provider::ConfigBuilder::default()
        .policy_set_path("tests/data/sweets.cedar")
        .build()
        .unwrap(),
).unwrap();
```

The policy provider requires only a path to the file containing the policies. The `cedar-local-agent` repository on GitHub includes [example policy files](https://github.com/cedar-policy/cedar-local-agent/tree/main/tests/data) that show the format.

Building the entity provider requires paths to both a JSON file for both entities and the schema:

```
let entity_provider = EntityProvider::new(
    entity_provider::ConfigBuilder::default()
         .entities_path("tests/data/sweets.entities.json")
         .schema_path("tests/data/sweets.schema.cedar.json")
         .build()
         .unwrap(),
).unwrap();
```

These providers are both supplied to the `new` constructor of `Authorizer` to build an `authorizer` function:

```
let authorizer: Authorizer<PolicySetProvider, EntityProvider> = 
  Authorizer::new(
    AuthorizerConfigBuilder::default()
        .entity_provider(Arc::new(entity_provider))
        .policy_set_provider(Arc::new(policy_set_provider))
        .build()
        .unwrap(),
);
```

The `authorizer` function can be used to make policy decisions by calling its `is_authorized` function:

```
authorizer
      .is_authorized(&Request::new(
          Some(format!("User::\"Mike\"").parse().unwrap()),
          Some(format!("Action::\"update\"").parse().unwrap()),
          Some(format!("Box::\"1\"").parse().unwrap()),
          Context::empty(),
      ), &Entities::empty())
      .await
      .unwrap()
      .decision(),
```

The preceding code creates a request with principal `User::"Mike"` taking action `Action::"update"` on a resource called `Box::"1"`. If evaluated with a policy set that contains the first policy in this post and `User::"Mike"` is a member of `Role::"boxManagers"` then the decision will be `Allow`. Whether or not `User::"Mike"` is considered a member of `Role::"boxManagers"` depends on the contents of the `sweets.entities.json` file that is referenced in the creation of the entity provider. The following excerpt from that file shows `User::"Mike"` to be a member of `Role::"boxManagers"`:

```
[
  ...
  {
    "uid": { "__entity": { "type": "User", "id": "Mike"} },
    "attrs": {},
    "parents": [
       {
         "type": "Role",
         "id": "boxManagers"
       }
    ]
  },
  ...
]
```

In this example, note that it does not require you to write any code that connects to and reads the policies or entities from the file system. They are automatically read and cached. The library also includes code for configuring the `Authorizer` to flush the policy and entity caches either on a periodic basis or by watching the file system for changes.

Adding logging is as simple as adding some additional configuration parameters when you create the `Authorizer` to say what to log and where to write the logs. This code configures a logger and then adds it when the `Authorizer` is constructed:

```
let log_config =
    log::ConfigBuilder::default()
        .field_set(log::FieldSetBuilder::default()
            .principal(true)
            .action(true)
            .resource(true)
            .context(true)
            .entities(log::FieldLevel::All)
            .build()
            .unwrap())
    .build()
    .unwrap();

let authorizer: Authorizer<PolicySetProvider, EntityProvider> = 
  Authorizer::new(
    AuthorizerConfigBuilder::default()
        .entity_provider(...)
        .policy_set_provider(...)
        .log_config(log_config)
        .build()
        .unwrap(),
);
```

Logging everything isn’t recommended because the access request might identify personal information and other sensitive data. For information about filtering logs for better security, see [Secure Logging Configuration](https://github.com/cedar-policy/cedar-local-agent#secure-logging-configuration).

## The avp-local-agent

Storing policies and entity information on disk works fine for a small number of policies that don’t change often. But as the number of policies grows or when they need to be periodically updated, storing them on the file system is not ideal. For many applications, policies are subject to an organizational governance process owned by a team outside the development organization. A robust policy management system provides a user interface for team members to work on policies and an API for integration with other parts of the organization’s identity governance infrastructure.

In addition, in an application that is deployed globally across multiple regions, customers may want a centralized policy store from which policies are downloaded, rather than multiple localized policy repositories that must be kept in sync. Finally, there are security considerations. Anyone who can change the policy and entity files can control access decisions. Developers building their own policy stores will have to verify they are secure to prevent unauthorized access.

[Amazon Verified Permissions Local Agent (`avp-local-agent`)](https://github.com/awslabs/avp-local-agent) is an open source Rust crate that solves the problem of building an easily accessible, available, robust, and secure policy store. The `avp-local-agent` builds on the `cedar-local-agent` discussed previously in this post, automatically reading policies and schema from [Amazon Verified Permissions](https://aws.amazon.com/verified-permissions/). Verified Permissions provides both a console-based and API-based experience that makes it easier for organizations to integrate policy management into their governance processes. Figure 3 shows how an application can use the `avp-local-agent`. In this figure, the application is using `avp-local-agent` to make access control decisions based on policies and schema stored in Verified Permissions.

![avp-local-agent architecture diagram](https://d2908q01vomqb2.cloudfront.net/ca3512f4dfa95a03169c5a670a4c91a19b3077b4/2023/12/13/avp-local-agent-diagram.png)

Figure 3: Using `avp-local-agent` in an application with Verified Permissions as the policy store.

## Managing Policies in the Cloud

[Verified Permissions](https://aws.amazon.com/verified-permissions/) provides cloud-based policy stores. Normally developers create one policy store per application (or tenant in a multi-tenant application). The Verified Permissions policy store has not only schema and policy editors, but also other policy management features for authoring and managing policies. These are available in the console and as an API.

## Using avp-local-agent

As was true with `cedar-local-agent`, using `avp-local-agent` requires creating an `Authorizer`, supplying policy and entity providers as parameters. However, with `avp-local-agent`, rather than providing a path to the files where they are stored, the providers are created with a reference to a policy store in Verified Permissions. To start, build a Verified Permissions client, specifying the AWS Region to use (`us-east-1` in this case):

```
let client = verified_permissions_default_credentials(
     Region::new("us-east-1")
  ).await;
```

This call assumes that the library can resolve the AWS credentials for the policy store used from one of the locations supported by the `DefaultCredentialsChain`.

This `client` and a policy store ID are used to create policy and entity providers that reference the application’s policy store. The following code examples assume that the policy store ID is `a1b2c3d4-5678-90ab-cdef-EXAMPLE11111`.

```
let policy_set_provider = 
    PolicySetProvider::from_client("a1b2c3d4-5678-90ab-cdef-EXAMPLE11111", 
                                   client.clone()
                                  ).unwrap();
```

```
let entity_provider =
    EntityProvider::from_client("a1b2c3d4-5678-90ab-cdef-EXAMPLE11111", 
                                client.clone()
                               ).unwrap();
```

The `EntityProvider` uses the policy store schema to populate action groups to match the functionality that Verified Permissions provides. Entity information about principals and resources can be added by building a custom entity provider or in the call to `is_authorized()` when the request is presented.

Similarly to the `cedar-local-agent`, you need to use the providers when you create an `Authorizer` which is used to evaluate access requests. To avoid repetition, it is not shown here. For an example of how to evaluate access requests, see the [README for the `avp-local-agent`](https://github.com/awslabs/avp-local-agent/blob/main/README.md).

## Conclusion

Cedar enables application developers to remove permissions logic from their application code, and instead express that logic as policies, using a domain specific language (DSL) designed for authorization. Separating the permissions logic from the application code in this way can improve the performance, security, and auditability of applications. The `cedar-local-agent` provides application developers who want to use the Cedar SDK with a localized store for their Cedar policies. Using `cedar-local-agent`, developers can take advantage of local policy evaluation without the burden of building the infrastructure around the Cedar SDK to manage policies and entities.

The `avp-local-agent` gives application developers the ability to manage policies centrally in the cloud with [Verified Permissions](https://aws.amazon.com/verified-permissions/), while continuing to evaluate them locally with the application. This combines the high performance and low transaction cost that comes from local evaluation, with the strong governance and security that comes from centralized management. Using Amazon Verified Permissions, customers can verify that only authorized users can create and modify policies, and see audit logs for all changes. Applications that create new policies at run time, for example when an admin creates a custom role, benefit from being able to use the service APIs to validate and store those policies. Applications that are deployed globally across multiple regions can still maintain a single centralized repository of policies.

If you’d like to learn more about how to evaluate policies locally with less development time, look at `cedar-local-agent` and try running the examples given in the `/test` directory. Similarly, look at the examples in the `avp-local-agent /tests` directory if you’d like the convenience, reliability, and flexibility of using cloud-managed policies in Verified Permissions with local evaluation.
