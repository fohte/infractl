use std::time::Duration;

use anyhow::Context;
use k8s_openapi::api::core::v1::{Pod, Secret};
use kube::api::{Api, ListParams, Portforwarder};
use kube::config::KubeConfigOptions;
use kube::{Client, Config};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::registry::Target;

/// Port a CloudNativePG instance pod serves Postgres on.
const POSTGRES_PORT: u16 = 5432;

const API_TIMEOUT: Duration = Duration::from_secs(10);

/// Build a client for the target's kube context, or the kubeconfig's
/// current-context when the target doesn't pin one.
pub async fn client(target: &Target) -> anyhow::Result<Client> {
    let options = KubeConfigOptions {
        context: target.context.clone(),
        ..Default::default()
    };
    let config = Config::from_kubeconfig(&options)
        .await
        .context("failed to load kubeconfig")?;

    Client::try_from(config).context("failed to build a kubernetes client")
}

/// Fetch the `password` field of the target's `kubernetes.io/basic-auth`
/// Secret, which holds the reader role's password.
pub async fn fetch_secret_password(client: &Client, target: &Target) -> anyhow::Result<String> {
    let secrets: Api<Secret> = Api::namespaced(client.clone(), &target.namespace);

    let secret = tokio::time::timeout(API_TIMEOUT, secrets.get(&target.secret_name))
        .await
        .context("timed out fetching the reader role's secret")?
        .with_context(|| format!("failed to get secret {}", target.secret_name))?;

    let password = secret
        .data
        .unwrap_or_default()
        .remove("password")
        .with_context(|| format!("secret {} has no password field", target.secret_name))?;

    String::from_utf8(password.0).context("secret password is not valid UTF-8")
}

/// Open a port-forward tunnel to the target cluster's primary instance pod and
/// take the Postgres stream out of it.
///
/// The returned handle owns the background task feeding the stream, so it has
/// to outlive the stream and be aborted once the stream is done with. Handing
/// out both at once keeps that cleanup obligation from starting before the
/// stream is in the caller's hands.
///
/// The stream is the API server's tunnel itself rather than a socket to a
/// locally bound port, so nothing listens on loopback while it is in use.
pub async fn open_postgres_tunnel(
    client: &Client,
    target: &Target,
) -> anyhow::Result<(
    Portforwarder,
    impl AsyncRead + AsyncWrite + Unpin + Send + use<>,
)> {
    let pod = find_primary_pod(client, target).await?;
    let pods: Api<Pod> = Api::namespaced(client.clone(), &target.namespace);

    let mut forwarder = tokio::time::timeout(API_TIMEOUT, pods.portforward(&pod, &[POSTGRES_PORT]))
        .await
        .context("timed out opening a port-forward tunnel")?
        .with_context(|| format!("failed to port-forward to pod {pod}"))?;

    let stream = forwarder
        .take_stream(POSTGRES_PORT)
        .context("port-forward tunnel exposed no stream for the Postgres port")?;

    Ok((forwarder, stream))
}

async fn find_primary_pod(client: &Client, target: &Target) -> anyhow::Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &target.namespace);
    let params = ListParams::default().labels(&primary_pod_selector(&target.cluster));

    let list = tokio::time::timeout(API_TIMEOUT, pods.list(&params))
        .await
        .context("timed out listing the cluster's instance pods")?
        .with_context(|| format!("failed to list pods for cluster {}", target.cluster))?;

    // CloudNativePG demotes the old primary before promoting a new one, so the
    // selector matches at most one pod even mid-failover.
    list.items
        .into_iter()
        .find_map(|pod| pod.metadata.name)
        .with_context(|| {
            format!(
                "no primary pod found for cluster {} in namespace {}",
                target.cluster, target.namespace
            )
        })
}

/// Selects the instance pod CloudNativePG currently promotes as primary.
fn primary_pod_selector(cluster: &str) -> String {
    format!("cnpg.io/cluster={cluster},cnpg.io/instanceRole=primary")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::plain("main", "cnpg.io/cluster=main,cnpg.io/instanceRole=primary")]
    #[case::hyphenated(
        "main-staging",
        "cnpg.io/cluster=main-staging,cnpg.io/instanceRole=primary"
    )]
    fn test_primary_pod_selector(#[case] cluster: &str, #[case] expected: &str) {
        assert_eq!(primary_pod_selector(cluster), expected);
    }
}
