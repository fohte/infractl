use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout, Command};

use crate::registry::Target;

const PORT_FORWARD_READY_TIMEOUT: Duration = Duration::from_secs(10);

/// Fetch and base64-decode the `password` field of the target's
/// `kubernetes.io/basic-auth` Secret via `kubectl get secret -o json`.
pub async fn fetch_secret_password(target: &Target) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct SecretResponse {
        data: SecretData,
    }

    #[derive(Deserialize)]
    struct SecretData {
        password: String,
    }

    let mut cmd = kubectl_command(target);
    cmd.args([
        "get",
        "secret",
        &target.secret_name,
        "-n",
        &target.namespace,
        "-o",
        "json",
    ]);

    let output = cmd
        .output()
        .await
        .with_context(|| format!("failed to run kubectl get secret {}", target.secret_name))?;
    if !output.status.success() {
        anyhow::bail!(
            "kubectl get secret {} failed: {}",
            target.secret_name,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let secret: SecretResponse = serde_json::from_slice(&output.stdout).with_context(|| {
        format!(
            "failed to parse kubectl get secret {} output",
            target.secret_name
        )
    })?;

    let decoded = BASE64
        .decode(secret.data.password)
        .context("failed to base64-decode secret password")?;

    String::from_utf8(decoded).context("secret password is not valid UTF-8")
}

/// A live `kubectl port-forward` subprocess tunnelling a local TCP port to
/// the target cluster's `-rw` service.
pub struct PortForward {
    child: Child,
    local_port: u16,
    // Keeps kubectl's stdout drained for the tunnel's whole lifetime. Once
    // the readiness line is found, nothing reads this pipe anymore unless
    // kept open here; letting it fill up (or dropping it, which closes our
    // read end) has been observed to break the proxied connection outright
    // rather than just losing log output.
    stdout_drain: tokio::task::JoinHandle<()>,
}

impl PortForward {
    pub async fn start(target: &Target) -> anyhow::Result<Self> {
        let mut cmd = kubectl_command(target);
        cmd.args([
            "port-forward",
            &format!("svc/{}-rw", target.cluster),
            ":5432",
            "-n",
            &target.namespace,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .context("failed to spawn kubectl port-forward")?;
        let stdout = child
            .stdout
            .take()
            .context("kubectl port-forward stdout was not piped")?;
        let mut lines = BufReader::new(stdout).lines();

        let local_port = tokio::time::timeout(
            PORT_FORWARD_READY_TIMEOUT,
            wait_for_forwarded_port(&mut lines),
        )
        .await
        .context("timed out waiting for kubectl port-forward to open a tunnel")??;

        let stdout_drain =
            tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });

        Ok(Self {
            child,
            local_port,
            stdout_drain,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Terminate the port-forward subprocess and wait for it to be reaped.
    pub async fn stop(mut self) -> anyhow::Result<()> {
        self.stdout_drain.abort();
        self.child
            .kill()
            .await
            .context("failed to stop kubectl port-forward")
    }
}

async fn wait_for_forwarded_port(lines: &mut Lines<BufReader<ChildStdout>>) -> anyhow::Result<u16> {
    while let Some(line) = lines
        .next_line()
        .await
        .context("failed to read kubectl port-forward output")?
    {
        if let Some(port) = parse_forwarded_port(&line) {
            return Ok(port);
        }
    }
    anyhow::bail!("kubectl port-forward exited before opening a tunnel")
}

/// Parses the local port out of a `kubectl port-forward` readiness line,
/// e.g. `Forwarding from 127.0.0.1:63421 -> 5432`. This isn't a documented
/// kubectl contract, but the format has been stable across versions.
fn parse_forwarded_port(line: &str) -> Option<u16> {
    line.strip_prefix("Forwarding from 127.0.0.1:")?
        .split(" -> ")
        .next()?
        .parse()
        .ok()
}

fn kubectl_command(target: &Target) -> Command {
    let mut cmd = Command::new("kubectl");
    if let Some(context) = &target.context {
        cmd.args(["--context", context]);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::ipv4("Forwarding from 127.0.0.1:63421 -> 5432", Some(63421))]
    #[case::ipv6_line_ignored("Forwarding from [::1]:63421 -> 5432", None)]
    #[case::unrelated_line("Handling connection for 5432", None)]
    #[case::empty_line("", None)]
    fn test_parse_forwarded_port(#[case] line: &str, #[case] expected: Option<u16>) {
        assert_eq!(parse_forwarded_port(line), expected);
    }
}
