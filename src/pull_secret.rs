use kube::CustomResource;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
#[kube(
group = "vsix.me",
kind = "PullSecret",
derive = "PartialEq",
derive = "Default",
version = "v1"
)]
pub struct PullSecretSpec {
    /// source is the secret object that will be copied.  It must be a dockerconfigjson secret.
    #[serde(rename = "source", skip_serializing_if = "Option::is_none")]
    pub source_secret: Option<PullSecretSource>,
    /// targetNamespaces is an array of namespaces that the source secret will be copied into.
    #[serde(rename = "targetNamespaces", skip_serializing_if = "Option::is_none")]
    pub target_namespaces: Option<Vec<String>>,

}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PullSecretSource {
    /// name is the name of the secret to be duplicated.
    #[serde(rename = "name", skip_serializing_if = "Option::is_none")]
    pub secret_name: Option<String>,
    /// namespace is the namespace which contains the source secret.
    #[serde(rename = "namespace", skip_serializing_if = "Option::is_none")]
    pub secret_namespace: Option<String>,

}
