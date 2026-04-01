use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let transport = TokioChildProcess::new(
        tokio::process::Command::new("cargo").configure(|cmd| {
            cmd.args(["run", "-q", "--bin", "server"]);
            cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
        }),
    )?;

    let mut client = ().serve(transport).await?;

    let args = json!({ "a": 1, "b": 999 });
    let params = CallToolRequestParams::new("add").with_arguments(
        args.as_object()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("arguments must be a JSON object"))?,
    );
    let info = client
        .peer_info()
        .ok_or_else(|| anyhow::anyhow!("server info missing after handshake"))?;
    println!("Server info: {:?}", info);
    let tools = client.list_tools(None).await?;
    println!("Tools: {:?}", tools);
    let result = client.call_tool(params).await?;
    println!("Result: {:?}", result.content);

    client.close().await?;
    Ok(())
}
