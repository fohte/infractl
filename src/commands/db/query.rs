mod kube;
mod output;
mod pg;

use clap::Args;

use crate::{config, registry};

/// Target used when `--target` is omitted.
const DEFAULT_TARGET_NAME: &str = "default";

#[derive(Args)]
pub struct QueryArgs {
    /// Target to query against. Defaults to the target named `default`.
    #[arg(short, long)]
    target: Option<String>,

    /// Emit structured JSON instead of a human-readable table.
    #[arg(long)]
    json: bool,

    /// SQL statement(s) to run.
    sql: String,
}

pub async fn run(args: &QueryArgs) -> anyhow::Result<()> {
    let config = config::load_config()?;
    let target_name = args.target.as_deref().unwrap_or(DEFAULT_TARGET_NAME);
    let target = registry::resolve(&config, target_name).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown target {target_name:?} (run `infractl db targets` to list configured targets)"
        )
    })?;

    let client = kube::client(&target).await?;
    let password = kube::fetch_secret_password(&client, &target).await?;
    let (port_forward, stream) = kube::open_postgres_tunnel(&client, &target).await?;

    let query_result = pg::run_query(stream, &target, &password, &args.sql).await;
    port_forward.abort();
    let result_sets = query_result?;

    if args.json {
        println!("{}", output::format_json(&result_sets)?);
    } else {
        print!("{}", output::format_table(&result_sets));
    }

    Ok(())
}
