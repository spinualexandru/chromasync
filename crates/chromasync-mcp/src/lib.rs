pub mod convert;
pub mod params;
pub mod tools;

use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool_handler,
};

#[derive(Clone)]
pub struct ChromasyncServer {
    tool_router: ToolRouter<Self>,
}

impl Default for ChromasyncServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for ChromasyncServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "chromasync-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Chromasync MCP server: generate color themes from seed colors or wallpaper images. \
                 Use list_templates, list_targets, and list_packs to discover available options, \
                 then generate or preview themes.",
            )
    }
}
