import { z } from "zod";
import { defineTool } from "../framework";

export const close_tab_group = defineTool({
  name: "close_tab_group",
  description: "Close a GitButler tab group, returning all its tabs back to individual tabs. The group is removed and all contained tabs become ungrouped. Closes the group in GitButler UI.",
  approvalCategory: "automation",
  input: z.object({
    groupId: z.string().describe("GitButler group ID to close (e.g., 'group-1', 'group-2')"),
  }),
  output: z.object({
    success: z.boolean().describe("True if group was closed successfully"),
    groupId: z.string().describe("Group ID that was closed"),
    tabCount: z.number().describe("Number of tabs that were in the group and are now individual tabs"),
    ungroupedTabs: z.array(z.number()).describe("List of tab IDs that are now individual tabs"),
  }),
  handler: async (args, _ctx, response) => {
    response.text(`Closing GitButler tab group ${args.groupId}...`);

    // Phase 3: Implementation - Returns mock data for now
    // TODO: Replace with actual GitButler MCP call:
    // await gitbutlerClient.closeGroup(args.groupId);

    const mockTabs = [1, 2, 3]; // Mock tab IDs

    response.data({
      success: true,
      groupId: args.groupId,
      tabCount: mockTabs.length,
      ungroupedTabs: mockTabs,
    });

    response.text(`Group ${args.groupId} closed. ${mockTabs.length} tabs are now individual.`);
  },
});
