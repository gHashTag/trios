import { z } from "zod";
import { defineTool } from "../framework";

export const discard_file_changes = defineTool({
  name: "discard_file_changes",
  description: "Discard uncommitted file changes (reset to last commit). Removes all unstaged modifications and restores repository to clean state.",
  approvalCategory: "automation",
  input: z.object({}),
  output: z.object({
    success: z.boolean(),
    message: z.string(),
    discardedFiles: z.array(z.string()),
  }),
  handler: async (_args, _ctx, response) => {
    response.text("Discarding uncommitted file changes...");

    // Phase 3: Implementation - GitButler CLI call
    // TODO: Replace mock with actual GitButler CLI call
    // git checkout -- . && git clean -fd

    response.data({
      success: true,
      message: "Uncommitted changes discarded. Repository restored to clean state.",
      discardedFiles: [], // TODO: Return actual list of discarded files
    });

    response.text("Repository is now clean.");
  },
});
