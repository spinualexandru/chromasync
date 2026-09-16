use std::{process::Stdio, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

struct Client {
    child: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
    modern: bool,
}

impl Client {
    fn new(modern: bool) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_chromasync-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()).lines(),
            child,
            modern,
        }
    }

    async fn send(&mut self, message: Value) {
        let mut bytes = serde_json::to_vec(&message).unwrap();
        bytes.push(b'\n');
        self.input.write_all(&bytes).await.unwrap();
    }

    async fn request(&mut self, id: u64, method: &str, mut params: Value) -> Value {
        if self.modern {
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": {"name": "chromasync-test", "version": "1"},
                "io.modelcontextprotocol/clientCapabilities": {},
            });
        }
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await;
        let line = timeout(Duration::from_secs(10), self.output.next_line())
            .await
            .expect("MCP response timed out")
            .unwrap()
            .expect("MCP server closed stdout");
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], id, "{response}");
        response
    }

    async fn check_tools(&mut self) {
        let listed = self.request(2, "tools/list", json!({})).await;
        let tools = listed["result"]["tools"].as_array().unwrap();
        let mut names: Vec<_> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "batch",
                "export_tokens",
                "generate",
                "generate_palette",
                "list_packs",
                "list_targets",
                "list_templates",
                "pack_info",
                "preview",
                "wallpaper"
            ]
        );
        let palette_tool = tools
            .iter()
            .find(|tool| tool["name"] == "generate_palette")
            .unwrap();
        assert_eq!(
            palette_tool["inputSchema"]["properties"]["seed"]["type"],
            "string"
        );
        assert!(
            palette_tool["inputSchema"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!("seed"))
        );

        let called = self
            .request(
                3,
                "tools/call",
                json!({
                    "name": "generate_palette", "arguments": {"seed": "#4ecdc4"},
                }),
            )
            .await;
        let result = &called["result"];
        assert_eq!(result["isError"], false, "{called}");
        assert_eq!(result["content"][0]["type"], "text");
        let palette: chromasync_types::GeneratedPalette =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(
            palette,
            chromasync_core::generate_palette(
                "#4ecdc4",
                chromasync_types::ThemeMode::Dark,
                chromasync_types::ChromaStrategy::Normal,
            )
            .unwrap()
        );
        if self.modern {
            assert_eq!(result["resultType"], "complete");
        } else {
            assert!(result.get("resultType").is_none());
        }

        let invalid = self
            .request(
                4,
                "tools/call",
                json!({
                    "name": "generate_palette", "arguments": {},
                }),
            )
            .await;
        assert_eq!(invalid["result"]["isError"], true, "{invalid}");
        assert!(
            invalid["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("missing field `seed`")
        );
    }
}

#[tokio::test]
async fn legacy_clients_can_initialize_and_call_tools() {
    let mut client = Client::new(false);
    let initialized = client
        .request(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "chromasync-test", "version": "1"},
            }),
        )
        .await;
    assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(
        initialized["result"]["serverInfo"]["name"],
        "chromasync-mcp"
    );
    assert_eq!(
        initialized["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(initialized["result"]["capabilities"]["tools"].is_object());
    client
        .send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;
    client.check_tools().await;
    client.child.kill().await.unwrap();
}

#[tokio::test]
async fn modern_clients_can_discover_and_call_tools() {
    let mut client = Client::new(true);
    let discovered = client.request(1, "server/discover", json!({})).await;
    assert!(
        discovered["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );
    assert!(discovered["result"]["capabilities"]["tools"].is_object());
    client.check_tools().await;
    client.child.kill().await.unwrap();
}
