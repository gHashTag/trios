import { logger } from "../lib/logger";
import { McpClient } from "@modelcontextprotocol/sdk/client.js";
import { loadConfig, type BridgeConfig } from "./config";
import { createBridgeServer, type BridgeDeps } from "./bridge-server";

// Import all tab_groups tools
import {
	discard_file_changes,
	list_tab_groups,
	group_tabs,
	update_tab_group,
	ungroup_tabs,
	close_tab_group,
} from "./tools/tab_groups/tab-groups-imports";

// Import all tab_groups tools as flat array
const tabGroupsTools = [
	discard_file_changes,
	list_tab_groups,
	group_tabs,
	update_tab_group,
	ungroup_tabs,
	close_tab_group,
];

async function main() {
	const config = loadConfig(parseArgs());

	logger.info("╔═════════════════════════════════════════════╗");
	logger.info("║                 TRIOS MCP Bridge — Vision + GitButler  ║");
	logger.info("╚═════════════════════════════════════════════════════╝");

	const mcpClient = new McpClient({
		name: "trios-mcp-bridge",
		version: config.version,
	});

	// Register all tab_groups tools with McpClient
	const toolNames = tabGroupsTools.map((t) => t.name);
	mcpClient.registerTools(toolNames);
	logger.info(`[Setup] Registered ${toolNames.length} tab_groups tools: ${toolNames.join(", ")}`);

	// Connect to GitButler MCP
	await mcpClient.connect(config.gitbutlerCliPath);
	logger.info(`[GitButler] Connected to CLI: ${config.gitbutlerCliPath}`);

	// Create bridge server
	const bridgeServer = createBridgeServer({ config, mcpClient });

	// Set up HTTP server with Hono
	const app = bridgeServer.app;

	// Health check endpoint
	app.get("/", (c) => {
		return c.json({
			name: "trios-mcp-bridge",
			version: config.version,
			status: "running",
			connections: {
				gitbutler: mcpClient.isConnected ? "connected" : "disconnected",
				trios_mcp: "active",
			},
			config: {
				port: config.port,
				gitbutlerCliPath: config.gitbutlerCliPath,
				workingDir: config.workingDir,
			},
			tools: {
				tab_groups: toolNames,
			},
		});
	});

	// Start server
	console.log(`\n🚀 TRIOS MCP Bridge running at http://127.0.0.1:${config.port}`);
	console.log(`   GitButler endpoint: ${config.gitbutlerCliPath}`);
	console.log(`   Health check: http://127.0.0.1:${config.port}/`);
	console.log(`   Health check: http://127.0.0.1:${config.port}/health`);
	console.log("\n   Press Ctrl+C to stop.\n");

	Bun.serve({
		port: config.port,
		fetch: app.fetch,
	});
}

main().catch((err) => {
	logger.error("Fatal error:", err);
	process.exit(1);
});
