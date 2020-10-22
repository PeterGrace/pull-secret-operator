use crate::pull_secret::{PullSecretSource, PullSecretSpec};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Secret;
use kube::api::{Meta, PostParams};
use kube::{
    api::{Api, ListParams},
    Client,
};
use kube_runtime::watcher;
use kube_runtime::watcher::Event;
use log::info;
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::{channel, Sender};
use tokio::time::{delay_for, Duration};

const PULL_TYPE: &str = "kubernetes.io/dockerconfigjson";
const NOT_NAMESPACED: &str = "not-namespaced";

#[derive(Error, Debug)]
pub enum PullSecretError {
    #[error("Unable to look up source secret {0} in namespace {1}.")]
    SourceError(String, String),
    #[error("Misc unknown: {0}")]
    Unknown(String),
    #[error("Secret {0}/{1} found, but it is not a docker pull secret.  It is a {2}")]
    InvalidType(String, String, String),
    #[error(transparent)]
    KubeError(#[from] kube::Error),
    #[error(transparent)]
    WatcherError(#[from] kube_runtime::watcher::Error),
    #[error("We've retried too many times to watch Secret {0}/{1}; cancelling further watches until next object reload")]
    TooManyRetries(String, String),
}

pub fn btree_to_string(input: BTreeMap<String, String>) -> String {
    let mut result = String::from("");
    for key in input.keys() {
        if result.len() == 0 {
            result = format!("{}={}", key, input.get(key).unwrap());
        } else {
            result = format!("{}, {}={}", result, key, input.get(key).unwrap());
        }
    }
    result
}

pub async fn create_and_start_watchers() -> Result<(), PullSecretError> {
    let mut watch_registry: HashMap<String, Vec<Sender<()>>> = HashMap::new();
    let client = Client::try_default().await?;
    let lp = ListParams::default().allow_bookmarks();
    //Watcher(Watcher),
    let watched: Api<crate::pull_secret::PullSecret> = Api::all(client);
    let mut w = watcher(watched, lp).boxed();
    while let Some(watcher_status) = w.try_next().await? {
        match watcher_status {
            Event::Applied(s) => {
                info!("Processing delete on Watcher: {}", Meta::name(&s));
                if let Some(watch_channels) = watch_registry.get_mut(&s.name()) {
                    for watch_channel in watch_channels {
                        let response = watch_channel.send(()).await;
                        match response {
                            Ok(_) => (),
                            Err(e) => info!("Received error from applied-webhook watcher: {}", e),
                        }
                    }
                }
                let mut watch_vec: Vec<Sender<()>> = Vec::new();
                let watcher_name = s.name();
                info!("Processing apply on Watcher: {}", &s.name());
                let (tx, mut rx) = channel(1);
                tokio::spawn(async move {
                    select! {
                     res = create_watch_from_spec(s.spec) => {
                         match res {
                            Ok(_) => info!("watcher exited for unknown non-error reason"),
                            Err(e) => info!("Error when watching: {}",e.to_string())
                         }
                     },

                     _ = rx.next() => info!("Received word we should exit")
                    }
                });
                watch_vec.push(tx);
                watch_registry.insert(watcher_name, watch_vec);
            }
            Event::Deleted(s) => {
                info!("Processing delete on Watcher: {}", &s.name());
                if let Some(watch_channels) = watch_registry.get_mut(&s.name()) {
                    for watch_channel in watch_channels {
                        let response = watch_channel.send(()).await;
                        match response {
                            Ok(_) => (),
                            Err(e) => info!("Received error from deleted-webhook watcher: {}", e),
                        }
                    }
                }
            }
            Event::Restarted(s) => {
                for object in s.iter() {
                    info!("Processing delete on Watcher: {}", &object.name());
                    // first, delete all preexisting watches for this object
                    if let Some(watch_channels) = watch_registry.get_mut(&object.name()) {
                        for watch_channel in watch_channels {
                            let response = watch_channel.send(()).await;
                            match response {
                                Ok(_) => (),
                                Err(e) => {
                                    info!("Received error from restarted-webhook watcher: {}", e)
                                }
                            }
                        }
                    }

                    // now, recreate all the watches
                    info!("Processing apply on Watcher: {}", &object.name());
                    let pss: PullSecretSpec = object.spec.clone();
                    let source_secret: PullSecretSource = pss.clone().source_secret.unwrap();
                    let mut watch_channels = Vec::new();
                    let id = format!(
                        "{}+{}",
                        source_secret.secret_namespace.unwrap(),
                        source_secret.secret_name.unwrap()
                    );
                    info!("Inserting channel into watch registry for key {}", id);
                    let (tx, mut rx) = channel(1);
                    tokio::spawn(async move {
                        select! {
                         res = create_watch_from_spec(pss) => {
                             match res {
                                Ok(_) => info!("watcher exited for unknown non-error reason"),
                                Err(e) => info!("Error when watching: {}",e.to_string())
                             }
                         },
                         _ = rx.next() => info!("Received word we should exit")
                        }
                    });
                    watch_channels.push(tx);

                    watch_registry.insert(object.name(), watch_channels);
                }
            }
        }
    }
    Ok(())
}
pub async fn create_watch_from_spec(input_obj: PullSecretSpec) -> Result<(), PullSecretError> {
    let s_ns = input_obj
        .source_secret
        .clone()
        .unwrap()
        .secret_namespace
        .unwrap();
    let s_name = input_obj
        .source_secret
        .clone()
        .unwrap()
        .secret_name
        .unwrap();

    let client = Client::try_default().await?;
    let ns_secrets: Api<Secret> = Api::namespaced(client, &s_ns);
    let lp = ListParams::default().fields(format!("metadata.name={}", s_name).as_str());
    let mut w = watcher(ns_secrets.clone(), lp).boxed();
    let mut retry_count: i32 = 0;
    loop {
        while let watcher_try = w.try_next().await {
            match watcher_try {
                Ok(Some(watcher_status)) => match watcher_status {
                    Event::Applied(s) => {
                        info!("applied: {}", Meta::name(&s));
                        let secret = ns_secrets.clone().get(Meta::name(&s).as_str()).await?;
                        copy_secret(secret, input_obj.target_namespaces.clone().unwrap()).await?;
                    }
                    Event::Deleted(s) => {
                        info!("deleted: {}", Meta::name(&s));
                        let secret = ns_secrets.clone().get(Meta::name(&s).as_str()).await?;
                        copy_secret(secret, input_obj.target_namespaces.clone().unwrap()).await?;
                    }
                    Event::Restarted(vs) => {
                        for s in vs {
                            info!("restarted: {}", Meta::name(&s));
                            if s.type_.clone().unwrap().ne(PULL_TYPE) {
                                return Err(PullSecretError::InvalidType(
                                    Meta::namespace(&s).unwrap_or(NOT_NAMESPACED.to_string()),
                                    Meta::name(&s),
                                    s.type_.unwrap(),
                                ));
                            }
                            let secret = ns_secrets.clone().get(Meta::name(&s).as_str()).await?;
                            copy_secret(secret, input_obj.target_namespaces.clone().unwrap())
                                .await?;
                        }
                    }
                },
                Ok(None) => {
                    return Err(PullSecretError::SourceError(s_ns, s_name));
                }
                Err(e) => {
                    info!("retry {}:Received error: {}", retry_count, e);
                    delay_for(Duration::from_secs(10)).await;
                    retry_count += 1;
                    if retry_count >= 30 {
                        return Err(PullSecretError::TooManyRetries(s_ns, s_name));
                    };
                }
            }
        }
    }
}

pub async fn copy_secret(
    source_secret: Secret,
    target_namespaces: Vec<String>,
) -> Result<(), PullSecretError> {
    for namespace in target_namespaces {
        info!(
            "Preparing to copy {}/{} to namespace {}",
            Meta::namespace(&source_secret).unwrap_or(NOT_NAMESPACED.to_string()),
            Meta::name(&source_secret),
            namespace
        );

        let client = Client::try_default().await?;
        let ns_secrets: Api<Secret> = Api::namespaced(client, namespace.as_str());

        let existing_secret = ns_secrets.get(source_secret.name().as_str()).await;
        match existing_secret {
            Ok(es) => {
                info!(
                    "Secret {}/{} already exists, updating.",
                    es.namespace().unwrap(),
                    es.name()
                );
                let mut updated_secret: Secret = es.clone();
                updated_secret.metadata.annotations = None;
                updated_secret.metadata.labels = None;
                updated_secret.data = source_secret.clone().data;
                updated_secret.metadata.namespace = Some(namespace);
                ns_secrets
                    .replace(es.name().as_str(), &PostParams::default(), &updated_secret)
                    .await?;
            }
            Err(err) => {
                info!("Secret did not already exist, creating.");
                let mut updated_secret: Secret = source_secret.clone();
                updated_secret.metadata.annotations = None;
                updated_secret.metadata.labels = None;
                updated_secret.metadata.resource_version = None;
                updated_secret.metadata.namespace = Some(namespace);
                ns_secrets
                    .create(&PostParams::default(), &updated_secret)
                    .await?;
            }
        }
    }
    Ok(())
}
