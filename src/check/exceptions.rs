//! Exceptions: obligations that excuse a deliberate gap instead of promising
//! behaviour. Each names its target and fails once the target is gone, so it
//! can't outlive the code it excuses.

use std::collections::BTreeMap;
use std::path::Path;

use impish_core::Obligation;

use crate::contract::escapes;

/// The active exceptions of primitive `name`, as tag and target, and a
/// problem for each whose target is outside the crate or no longer exists.
pub fn excused(
    root: &Path,
    name: &str,
    ids: &BTreeMap<u32, Obligation>,
) -> (Vec<(String, String)>, Vec<String>) {
    let mut excused = Vec::new();
    let mut problems = Vec::new();
    for (id, o) in ids.iter().filter(|(_, o)| !o.retired) {
        let Some(target) = &o.target else { continue };
        let tag = format!("{name}/A{id}");
        let (file, func) = match target.split_once("::") {
            Some((file, func)) => (file, Some(func)),
            None => (target.as_str(), None),
        };
        if escapes(file) {
            problems.push(format!(
                "{tag} excuses `{target}`, which is outside the crate"
            ));
            continue;
        }
        let exists = match func {
            Some(func) => super::defines(root, file, func),
            None => root.join(file).is_file(),
        };
        if !exists {
            problems.push(format!("{tag} excuses `{target}`, which no longer exists"));
        }
        excused.push((tag, target.clone()));
    }
    (excused, problems)
}
