//! Re-checking a record's conditions against the project as it is now.

use std::fs;
use std::path::{Component, Path};
use std::process::Command;

use semver::{Version, VersionReq};
use toml::{Table, Value};

use super::{Check, Condition};

pub enum Verdict {
    Ok,
    /// Why the condition no longer holds, or can't be shown to.
    Suspect(String),
    /// Only a person can judge it.
    Unchecked,
}

pub fn evaluate(root: &Path, condition: &Condition) -> Verdict {
    match &condition.check {
        None => Verdict::Unchecked,
        Some(Check::Dep { name, version }) => dep(root, name, version.as_deref()),
        Some(Check::Env(var)) if std::env::var_os(var).is_some() => Verdict::Ok,
        Some(Check::Env(var)) => Verdict::Suspect(format!("${var} is not set")),
        Some(Check::File(path)) => file(root, path),
        Some(Check::Rustc(req)) => rustc(req),
    }
}

fn requirement(req: &str) -> Result<VersionReq, Verdict> {
    VersionReq::parse(req)
        .map_err(|e| Verdict::Suspect(format!("version requirement `{req}` doesn't parse: {e}")))
}

fn dep(root: &Path, name: &str, version: Option<&str>) -> Verdict {
    let req = match version.map(requirement).transpose() {
        Ok(req) => req,
        Err(verdict) => return verdict,
    };
    let Some(lock) = fs::read_to_string(root.join("Cargo.lock"))
        .ok()
        .and_then(|text| toml::from_str::<Table>(&text).ok())
    else {
        return Verdict::Suspect("no readable Cargo.lock".to_string());
    };
    let versions: Vec<&str> = lock
        .get("package")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_table)
        .filter(|p| p.get("name").and_then(Value::as_str) == Some(name))
        .filter_map(|p| p.get("version").and_then(Value::as_str))
        .collect();
    match req {
        _ if versions.is_empty() => Verdict::Suspect(format!("Cargo.lock has no {name}")),
        None => Verdict::Ok,
        Some(req)
            if versions
                .iter()
                .any(|v| Version::parse(v).is_ok_and(|v| req.matches(&v))) =>
        {
            Verdict::Ok
        }
        Some(req) => Verdict::Suspect(format!(
            "Cargo.lock has {name} {}, not {req}",
            versions.join(", ")
        )),
    }
}

/// A path under the root that exists; one that could lead outside it is
/// suspect before anything is read.
fn file(root: &Path, path: &str) -> Verdict {
    let rel = Path::new(path);
    let leaves = rel.has_root()
        || rel
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_)));
    if leaves {
        return Verdict::Suspect(format!("`{path}` leaves the project"));
    }
    match (root.join(rel).canonicalize(), root.canonicalize()) {
        (Ok(real), Ok(base)) if real.starts_with(&base) => Verdict::Ok,
        (Ok(_), Ok(_)) => Verdict::Suspect(format!("`{path}` leaves the project")),
        _ => Verdict::Suspect(format!("`{path}` doesn't exist")),
    }
}

fn rustc(req: &str) -> Verdict {
    let req = match requirement(req) {
        Ok(req) => req,
        Err(verdict) => return verdict,
    };
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Ok(out) = Command::new(rustc).arg("--version").output() else {
        return Verdict::Suspect("rustc doesn't run".to_string());
    };
    let text = String::from_utf8_lossy(&out.stdout);
    // A nightly's `-nightly` would keep it from matching any plain requirement.
    let Some(version) = text
        .split_whitespace()
        .nth(1)
        .and_then(|v| Version::parse(v).ok())
        .map(|v| Version::new(v.major, v.minor, v.patch))
    else {
        return Verdict::Suspect(format!("no version in `{}`", text.trim()));
    };
    if req.matches(&version) {
        Verdict::Ok
    } else {
        Verdict::Suspect(format!("rustc is {version}, not {req}"))
    }
}
