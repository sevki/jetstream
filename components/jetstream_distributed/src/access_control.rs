//! Access Control provides the ability to define access control lists for JetStream services.
//! This module is inspired by the Zanzibar model.
//! ```mermaid
//! graph TD
//!    subgraph "Object Namespaces"
//!        Doc[/"doc namespace"/]
//!        Repo[/"repo namespace"/]
//!        Team[/"team namespace"/]
//!        Org[/"org namespace"/]
//!    end
//!
//!    subgraph "Relations/Verbs"
//!        DocRels["Document Relations<br/>- owner<br/>- editor<br/>- viewer"]
//!        RepoRels["Repository Relations<br/>- admin<br/>- write<br/>- read"]
//!        TeamRels["Team Relations<br/>- owner<br/>- member"]
//!        OrgRels["Organization Relations<br/>- owner<br/>- member<br/>- billing"]
//!    end
//!
//!    subgraph "Example Tuples"
//!        T1["doc:design#editor@user123"]
//!        T2["repo:main#write@team:eng#member"]
//!        T3["team:eng#member@user456"]
//!        T4["org:branch#owner@user789"]
//!        T5["repo:main#read@org:branch#member"]
//!    end
//!
//!    subgraph "Userset Rewrites"
//!        R1["repo viewer includes repo editor"]
//!        R2["org member includes org owner"]
//!        R3["repo admin includes repo write"]
//!    end
//!
//!    %% Connect namespaces to their relations
//!    Doc --> DocRels
//!    Repo --> RepoRels
//!    Team --> TeamRels
//!    Org --> OrgRels
//!
//!    %% Example relationships
//!    DocRels --> T1
//!    RepoRels --> T2
//!    TeamRels --> T3
//!    OrgRels --> T4
//!    RepoRels --> T5
//!
//!    %% Relationship inheritance
//!    R1 -.->|"inherits"| RepoRels
//!    R2 -.->|"inherits"| OrgRels
//!    R3 -.->|"inherits"| RepoRels
//!
//!    %% Group membership flow example
//!    T3 -.->|"affects"| T2
//!    T4 -.->|"affects"| T5
//!
//!    classDef namespace fill:#e1d5e7,stroke:#9673a6
//!    classDef relation fill:#dae8fc,stroke:#6c8ebf
//!    classDef tuple fill:#d5e8d4,stroke:#82b366
//!    classDef rewrite fill:#fff2cc,stroke:#d6b656
//!
//!    class Doc,Repo,Team,Org namespace
//!    class DocRels,RepoRels,TeamRels,OrgRels relation
//!    class T1,T2,T3,T4,T5 tuple
//!    class R1,R2,R3 rewrite
//!
//!    %% Add descriptive notes
//!    style Doc fill:#f9f,stroke:#333
//!    style Repo fill:#f9f,stroke:#333
//!    style Team fill:#f9f,stroke:#333
//!    style Org fill:#f9f,stroke:#333
//! ```

use cel_interpreter::{Context, Program};

/// Access Control trait. This follows the zanzibar model, of subject, verb, resource.
#[trait_variant::make(Send+Sync)]
pub trait AccessControl {
    /// Check if a subject has access to a resource.
    async fn check(
        &self,
        subject: impl Subject,
        access: impl Verb,
        resource: impl Resource,
    ) -> bool;
}
/// Subject, an entity that can perform actions, could be a person, a service, a bot or an organization with members.
#[trait_variant::make(Send+Sync)]
pub trait Subject {
    /// The unique identifier for the subject.
    fn id(&self) -> String;
    /// Convert the subject into a CEL value.
    fn into_value(self) -> cel_interpreter::Value;
}

/// Resource, an entity that is acted upon, could be a file, a database, a service, a repository or a document.
#[trait_variant::make(Send+Sync)]
pub trait Resource {
    /// The unique identifier for the resource.
    fn id(&self) -> String;
    /// Convert the resource into a CEL value.
    fn into_value(self) -> cel_interpreter::Value;
}

/// Verb, an action that a subject can perform on a resource, could be read, write, delete, create, update, etc.
#[trait_variant::make(Send+Sync)]
pub trait Verb {
    /// The unique identifier for the verb.
    fn into_value(self) -> cel_interpreter::Value;
}

/// A simple ACL script that uses CEL to evaluate access control rules.
/// ```js
/// subject.id == "alice" && action == "write" && resource == "file1"
/// ```
pub struct Script {
    program: String,
}

impl AccessControl for Script {
    async fn check(
        &self,
        subject: impl Subject,
        access: impl Verb,
        resource: impl Resource,
    ) -> bool {
        let mut context = Context::default();
        context
            .add_variable("subject", subject.into_value())
            .unwrap();
        context.add_variable("action", access.into_value()).unwrap();
        context
            .add_variable("resource", resource.into_value())
            .unwrap();
        let program = Program::compile(&self.program).unwrap();

        let value = program.execute(&context);
        match value {
            Ok(cel_interpreter::Value::Bool(b)) => b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod test {
    use {cel_interpreter::objects::Map, okstd::prelude::*, std::collections::HashMap};

    use super::*;

    struct SimpleSubject {
        id: String,
    }
    struct SimpleResource {
        id: String,
    }
    enum SimpleVerb {
        Write,
    }
    impl Subject for SimpleSubject {
        fn id(&self) -> String {
            self.id.clone()
        }
        fn into_value(self) -> cel_interpreter::Value {
            let mut map: HashMap<cel_interpreter::objects::Key, cel_interpreter::objects::Value> =
                HashMap::new();
            map.insert(
                "id".to_string().into(),
                cel_interpreter::Value::String(self.id.clone().into()),
            );

            let map = Map { map: map.into() };

            cel_interpreter::Value::Map(map)
        }
    }
    impl Resource for SimpleResource {
        fn id(&self) -> String {
            self.id.clone()
        }
        fn into_value(self) -> cel_interpreter::Value {
            cel_interpreter::Value::String(self.id.clone().into())
        }
    }
    impl Verb for SimpleVerb {
        fn into_value(self) -> cel_interpreter::Value {
            match self {
                SimpleVerb::Write => cel_interpreter::Value::String("write".to_string().into()),
            }
        }
    }

    #[okstd::test]
    async fn test_simple_acl() {
        let script = Script {
            program: r#"subject.id == "alice" && action == "write" && resource == "file1""#
                .to_string(),
        };
        let subject = SimpleSubject {
            id: "alice".to_string(),
        };
        let resource = SimpleResource {
            id: "file1".to_string(),
        };
        let verb = SimpleVerb::Write;
        assert!(script.check(subject, verb, resource).await);
    }
}
