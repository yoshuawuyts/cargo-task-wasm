use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The cargo.toml representation for tasks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CargoToml {
    pub tasks: BTreeMap<String, TaskDetail>,
}

/// When definition of a task is more than just a version string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct TaskDetail {
    /// This path is usually relative to the crate's manifest, but when using workspace inheritance, it may be relative to the workspace!
    ///
    /// When calling [`Manifest::complete_from_path_and_workspace`] use absolute path for the workspace manifest, and then this will be corrected to be an absolute
    /// path when inherited from the workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Permissions and capabilities associated with the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
}

/// Permissions and capabilities associated with the task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    /// Decide whether to inherit all, none, or a white list of the environment
    /// variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherit_env: Option<InheritEnv>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(untagged)]
pub enum InheritEnv {
    Bool(bool),
    AllowList(Vec<String>),
}
