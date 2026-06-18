use anyhow::Result;

use crate::cli::Cli;
use crate::client::FabricClient;
use crate::output;

pub(super) async fn list_user_access(
    cli: &Cli,
    client: &FabricClient,
    user_id: &str,
) -> Result<()> {
    let resp = client
        .get_list(
            &format!("/admin/users/{user_id}/access"),
            "accessEntities",
            cli.all,
            cli.continuation_token.as_deref(),
        )
        .await?;

    output::render_list_with_token(
        cli,
        &resp.items,
        &["id", "displayName", "type", "accessDetails"],
        &["ID", "NAME", "TYPE", "ACCESS"],
        "id",
        resp.continuation_token.as_deref(),
    );
    Ok(())
}
