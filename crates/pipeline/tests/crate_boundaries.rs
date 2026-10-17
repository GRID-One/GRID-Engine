//! The structural crate-boundary test (engine-spec §8.1 rules 1, 3, 4 and 5; ADR-014;
//! `docs/05-model-specs/synthetic-world.md` §10.1; P1-01 "Structural test").
//!
//! It runs `cargo metadata --format-version 1 --offline --locked` with the cargo that built this
//! test, parses the result with `serde_json`, and asserts every boundary rule below, printing
//! each rule as it checks it so the test log is the evidence.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// The thirteen workspace members of the DR-A8 crate set (P1-01, "Rust workspace").
const EXPECTED_MEMBERS: [&str; 13] = [
    "grid-domain",
    "grid-persistence",
    "grid-ingestion",
    "grid-identity",
    "grid-features",
    "grid-models",
    "grid-simulation",
    "grid-scoring",
    "grid-evaluation",
    "grid-governance",
    "grid-pipeline",
    "grid-cli",
    "grid-synth",
];

/// The numerical crates that stay synchronous (engine-spec §8.1 rule 3; synthetic-world §10.1).
const NUMERICAL_CRATES: [&str; 4] = [
    "grid-models",
    "grid-simulation",
    "grid-scoring",
    "grid-synth",
];

/// Python binding, embedding and interpreter crates that may appear nowhere in the resolved graph
/// (engine-spec §1.1 item 8, §8.1 rule 1). Matched as a substring of the package name.
const PYTHON_CRATE_PATTERNS: [&str; 6] = [
    "pyo3",
    "cpython",
    "python3-sys",
    "rustpython",
    "inline-python",
    "pyo3-ffi",
];

struct Workspace {
    metadata: Value,
}

impl Workspace {
    fn load() -> Self {
        let cargo = env!("CARGO");
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let output = Command::new(cargo)
            .args(["metadata", "--format-version", "1", "--offline", "--locked"])
            .current_dir(manifest_dir)
            .output()
            .expect("cargo metadata runs");
        assert!(
            output.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: Value =
            serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
        Self { metadata }
    }

    fn packages(&self) -> &Vec<Value> {
        self.metadata["packages"]
            .as_array()
            .expect("metadata.packages is an array")
    }

    fn package_by_id(&self, id: &str) -> &Value {
        self.packages()
            .iter()
            .find(|p| p["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("package id {id} is in metadata.packages"))
    }

    fn package_by_name(&self, name: &str) -> &Value {
        self.packages()
            .iter()
            .find(|p| p["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("package {name} is in metadata.packages"))
    }

    fn member_names(&self) -> BTreeSet<String> {
        self.metadata["workspace_members"]
            .as_array()
            .expect("metadata.workspace_members is an array")
            .iter()
            .map(|id| {
                self.package_by_id(id.as_str().expect("member id is a string"))["name"]
                    .as_str()
                    .expect("package name is a string")
                    .to_owned()
            })
            .collect()
    }

    /// The declared dependencies of `package` of the given kind (`None` is a normal dependency,
    /// `Some("dev")`, `Some("build")`), by crate name.
    fn declared_deps(&self, package: &str, kind: Option<&str>) -> BTreeSet<String> {
        self.package_by_name(package)["dependencies"]
            .as_array()
            .expect("package.dependencies is an array")
            .iter()
            .filter(|d| d["kind"].as_str() == kind)
            .map(|d| d["name"].as_str().expect("dependency name").to_owned())
            .collect()
    }

    /// The resolved node (`metadata.resolve.nodes[*]`) of the package named `name`.
    fn resolve_node(&self, name: &str) -> &Value {
        let id = self.package_by_name(name)["id"]
            .as_str()
            .expect("package id is a string");
        self.metadata["resolve"]["nodes"]
            .as_array()
            .expect("metadata.resolve.nodes is an array")
            .iter()
            .find(|n| n["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("package {name} has a resolve node"))
    }

    /// The transitive closure of `package`'s normal and build dependencies over the resolved
    /// graph (`metadata.resolve.nodes[*].deps`, each edge filtered to a `dep_kinds` entry whose
    /// `kind` is normal (`null`) or `build`; dev-only edges are not followed), by package name,
    /// excluding `package` itself. A crate that reaches `tokio` through `grid-persistence` is
    /// caught here where the declared-dependency check cannot see it.
    fn transitive_deps(&self, package: &str) -> BTreeSet<String> {
        let mut closure = BTreeSet::new();
        let mut frontier = vec![package.to_owned()];
        while let Some(name) = frontier.pop() {
            let node = self.resolve_node(&name);
            for dep in node["deps"].as_array().expect("node.deps is an array") {
                let followed = dep["dep_kinds"]
                    .as_array()
                    .expect("dep.dep_kinds is an array")
                    .iter()
                    .any(|k| matches!(k["kind"].as_str(), None | Some("build")));
                if !followed {
                    continue;
                }
                let dep_name = self.package_by_id(dep["pkg"].as_str().expect("dep.pkg"))["name"]
                    .as_str()
                    .expect("package name")
                    .to_owned();
                if closure.insert(dep_name.clone()) {
                    frontier.push(dep_name);
                }
            }
        }
        closure
    }

    /// Every package name in the resolved graph (`metadata.resolve.nodes`), plus every package
    /// cargo listed at all, so that a crate reachable by any edge is covered.
    fn resolved_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self
            .packages()
            .iter()
            .map(|p| p["name"].as_str().expect("package name").to_owned())
            .collect();
        if let Some(nodes) = self.metadata["resolve"]["nodes"].as_array() {
            for node in nodes {
                let id = node["id"].as_str().expect("node id");
                names.insert(self.package_by_id(id)["name"].as_str().unwrap().to_owned());
            }
        }
        names
    }
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn crate_boundaries_hold() {
    let ws = Workspace::load();

    println!("rule: the workspace has exactly the 13 DR-A8 members, every one named grid-*");
    let members = ws.member_names();
    assert_eq!(
        members,
        set(&EXPECTED_MEMBERS),
        "workspace members differ from the DR-A8 set"
    );
    for name in &members {
        assert!(
            name.starts_with("grid-"),
            "member {name} is not named grid-*"
        );
    }
    println!(
        "  ok: {}",
        members.iter().cloned().collect::<Vec<_>>().join(", ")
    );

    println!("rule: grid-domain's normal dependencies are exactly {{serde}} (ADR-014 decision 1)");
    let domain_deps = ws.declared_deps("grid-domain", None);
    assert_eq!(
        domain_deps,
        set(&["serde"]),
        "grid-domain normal dependencies: {domain_deps:?}"
    );
    println!("  ok: {domain_deps:?}");

    println!(
        "rule: grid-cli's normal dependencies are exactly {{grid-pipeline, grid-domain}} \
         (engine-spec §8.1 rule 4)"
    );
    let cli_deps = ws.declared_deps("grid-cli", None);
    assert_eq!(
        cli_deps,
        set(&["grid-pipeline", "grid-domain"]),
        "grid-cli normal dependencies: {cli_deps:?}"
    );
    println!("  ok: {cli_deps:?}");

    println!(
        "rule: grid-domain has no build dependency and no dev dependency (ADR-014 decision 1: \
         serde \"and nothing else outside std\"; a build script or a test could otherwise pull \
         a second external crate)"
    );
    for kind in ["build", "dev"] {
        let deps = ws.declared_deps("grid-domain", Some(kind));
        assert!(
            deps.is_empty(),
            "grid-domain has {kind} dependencies: {deps:?}"
        );
        println!("  ok: {kind} dependencies {deps:?}");
    }

    println!(
        "rule: grid-cli has no build dependency, and its dev dependencies are within \
         {{grid-pipeline, grid-domain}} (engine-spec §8.1 rule 4: \"MUST depend only on the \
         orchestration crate (and domain for DTOs)\")"
    );
    let cli_build = ws.declared_deps("grid-cli", Some("build"));
    assert!(
        cli_build.is_empty(),
        "grid-cli has build dependencies: {cli_build:?}"
    );
    let cli_dev = ws.declared_deps("grid-cli", Some("dev"));
    assert!(
        cli_dev.is_subset(&set(&["grid-pipeline", "grid-domain"])),
        "grid-cli dev dependencies: {cli_dev:?}"
    );
    println!("  ok: build dependencies {cli_build:?}, dev dependencies {cli_dev:?}");

    println!(
        "rule: grid-cli reaches, transitively over normal and build edges, nothing that \
         grid-pipeline does not already reach (engine-spec §8.1 rule 4: no argument-parser, \
         runtime or other crate enters the engine through the CLI)"
    );
    let cli_closure = ws.transitive_deps("grid-cli");
    let mut through_pipeline = ws.transitive_deps("grid-pipeline");
    through_pipeline.insert("grid-pipeline".to_owned());
    let cli_only: BTreeSet<String> = cli_closure.difference(&through_pipeline).cloned().collect();
    assert!(
        cli_only.is_empty(),
        "grid-cli reaches packages grid-pipeline does not: {cli_only:?}"
    );
    println!(
        "  ok: grid-cli normal+build closure of {} packages is within grid-pipeline's: {cli_closure:?}",
        cli_closure.len()
    );

    println!(
        "rule: no numerical crate (models, simulation, scoring, synth) has a normal dependency \
         on tokio or sqlx (engine-spec §8.1 rule 3)"
    );
    for crate_name in NUMERICAL_CRATES {
        let deps = ws.declared_deps(crate_name, None);
        for forbidden in ["tokio", "sqlx"] {
            assert!(
                !deps.contains(forbidden),
                "{crate_name} has a normal dependency on {forbidden}"
            );
        }
        println!("  ok: {crate_name} normal dependencies {deps:?}");
    }

    println!(
        "rule: no numerical crate reaches tokio or sqlx through any normal or build dependency, \
         transitively (engine-spec §8.1 rule 3: \"no Tokio or SQLx dependency\")"
    );
    for crate_name in NUMERICAL_CRATES {
        let closure = ws.transitive_deps(crate_name);
        for forbidden in ["tokio", "sqlx"] {
            assert!(
                !closure.contains(forbidden),
                "{crate_name} reaches {forbidden} transitively; closure: {closure:?}"
            );
        }
        println!(
            "  ok: {crate_name} normal+build closure of {} packages: {closure:?}",
            closure.len()
        );
    }

    println!(
        "rule: grid-models has no normal or build dependency on grid-synth, declared or \
         transitive (synthetic-world.md §10.1; engine-spec §8.1 rule 3)"
    );
    for kind in [None, Some("build")] {
        let deps = ws.declared_deps("grid-models", kind);
        assert!(
            !deps.contains("grid-synth"),
            "grid-models has a {} dependency on grid-synth",
            kind.unwrap_or("normal")
        );
    }
    let models_closure = ws.transitive_deps("grid-models");
    assert!(
        !models_closure.contains("grid-synth"),
        "grid-models reaches grid-synth transitively; closure: {models_closure:?}"
    );
    println!(
        "  ok: grid-models normal+build closure of {} packages: {models_closure:?}",
        models_closure.len()
    );

    println!(
        "rule: no package in the resolved graph is a Python binding, embedding or interpreter \
         crate (engine-spec §8.1 rule 1)"
    );
    let resolved = ws.resolved_names();
    assert!(
        resolved.len() >= members.len(),
        "resolved graph is smaller than the member set"
    );
    for name in &resolved {
        for pattern in PYTHON_CRATE_PATTERNS {
            assert!(
                !name.contains(pattern),
                "package {name} in the resolved graph matches the Python crate pattern {pattern}"
            );
        }
    }
    println!(
        "  ok: {} packages checked against {:?}",
        resolved.len(),
        PYTHON_CRATE_PATTERNS
    );
}
