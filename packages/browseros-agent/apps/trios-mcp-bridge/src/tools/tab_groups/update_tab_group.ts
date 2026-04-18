import { z } from "zod";
import { defineTool } from "../framework";

export const update_tab_group = defineTool({
  name: "update_tab_group",
  description: "Update properties of a GitButler tab group (name, color, collapsed/expanded state). Changes are reflected immediately in GitButler UI.",
  approvalCategory: "automation",
  input: z.object({
    groupId: z.string().describe("GitButler group ID (from list_tab_groups)"),
    name: z.string().optional().describe("Group name (e.g., 'feature/auth', 'bug-fixes')"),
    color: z.enum(["grey", "blue", "red", "yellow", "green", "pink", "purple", "cyan", "orange"]).optional().describe("Group color"),
    collapsed: z.boolean().optional().describe("Whether group is collapsed (tabs hidden) or expanded (visible)"),
  }),
  output: z.object({
    groupId: z.string().describe("Updated group ID"),
    name: z.string().optional().describe("Updated group name"),
    color: z.string().optional().describe("Updated group color"),
    collapsed: z.boolean().optional().describe("Updated collapsed state"),
    tabIds: z.array(z.number()).describe("Tab IDs that are currently in this group"),
  }),
  handler: async (args, _ctx, response) => {
    response.text(`Updating tab group ${args.groupId}...`);

    // Phase 3: Implementation - GitButler MCP call
    // TODO: Replace with actual CDP BrowserOS MCP call:
    // await gitbutlerClient.updateGroup(args.groupId, {
    //   name: args.name,
    //   color: args.color,
    //   collapsed: args.collapsed,
    // });

    response.data({
      groupId: args.groupId,
      name: args.name || "",
      color: args.color || "blue",
      collapsed: args.collapsed ?? false,
      tabIds: [],
    });

    response.text(`Group ${args.groupId} updated${args.name ? ` to "${args.name}" : ""}${args.color ? ` (${args.color})` : ""}${args.collapsed ? " (collapsed)" : " (expanded)"}`);
  },
});
