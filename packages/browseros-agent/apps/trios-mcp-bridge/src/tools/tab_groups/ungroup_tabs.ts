import { z } from "zod";
import { defineTool } from "../framework";

export const ungroup_tabs = defineTool({
  name: "ungroup_tabs",
  description: "Remove all tabs from a GitButler tab group, returning them to individual tabs. Ungrouped tabs can be regrouped later.",
  approvalCategory: "automation",
  input: z.object({
    groupId: z.string().describe("GitButler group ID to ungroup (e.g., 'group-1')"),
  }),
  output: z.object({
    success: z.boolean().describe("True if all tabs were successfully ungrouped"),
    tabIds: z.array(z.number()).describe("Tab IDs that were previously in this group and are now individual tabs"),
    groupId: z.string().describe("Group ID that was removed"),
    tabCount: z.number().describe("Number of tabs that were ungrouped"),
  }),
  handler: async (args, _ctx, response) => {
    response.text(`Ungrouping ${args.groupId}...`);

    // Phase 3: Implementation - Returns mock data for now
    // TODO: Replace with actual GitButler MCP call:
    // await gitbutlerClient.ungroupTabs(args.groupId);

    response.data({
      success: true,
      tabIds: [1, 2, 3], // Mock data
      groupId: args.groupId,
      tabCount: 3,
    });

    response.text(`Group ${args.groupId} removed. 3 tabs ungrouped.`);
  },
});
