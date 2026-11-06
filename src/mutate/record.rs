//! What `mutate --write` leaves in contract/coverage.json for `check`.

use std::fs;
use std::path::Path;

use anyhow::Context;
use serde_json::{Value, json};

use crate::contract::fingerprint;

/// Records in contract/coverage.json whether the primitive's mutate is met,
/// with the fingerprint `check` holds it to.
pub(super) fn record(
    root: &Path,
    primitive: &str,
    covered: Vec<String>,
    met: bool,
) -> anyhow::Result<()> {
    let path = root.join("contract/coverage.json");
    let mut doc: Value = if path.exists() {
        let text = fs::read_to_string(&path).context("read contract/coverage.json")?;
        serde_json::from_str(&text).context("read contract/coverage.json")?
    } else {
        json!({})
    };
    let fingerprint = fingerprint(primitive, covered, |file| {
        fs::read_to_string(root.join(file)).unwrap_or_default()
    });
    doc["primitives"][primitive]["mutate"] = json!({"met": met, "fingerprint": fingerprint});
    fs::write(&path, serde_json::to_string_pretty(&doc)? + "\n")
        .context("write contract/coverage.json")
}
