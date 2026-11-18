/**
 * bulkhead-orch — Pi extension for bulkhead.
 *
 * Transport and guard only. It registers `bh_*` tools that shell out to the
 * `bh` CLI, and nothing else: no classification, scheduling or file watching
 * lives here. Those belong to `bh-core` and `bhd`.
 *
 * Tested against Pi 1.5.0. Pi APIs used (see extensions.md):
 *   - `pi.registerTool()` for the `bh_*` tools.
 *   - `pi.exec()` to run `bh` without a shell.
 *
 * Wake delivery and the turn-end guard arrive in later units.
 */
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { StringEnum } from "@earendil-works/pi-ai";
import { Type, type TSchema } from "typebox";

const BH_BIN = process.env.BULKHEAD_BH_BIN ?? "bh";

export default function bulkheadOrch(pi: ExtensionAPI): void {
  registerBhTools(pi);
}

function registerBhTools(pi: ExtensionAPI): void {
  const register = (
    name: string,
    description: string,
    snippet: string,
    parameters: TSchema,
    toArgs: (params: Record<string, unknown>) => string[],
  ): void => {
    pi.registerTool({
      name,
      label: name,
      description,
      promptSnippet: snippet,
      parameters,
      async execute(_toolCallId, params, signal) {
        const args = toArgs(params as Record<string, unknown>);
        const result = await pi.exec(BH_BIN, args, { signal });
        const stream = result.code === 0 ? result.stdout : result.stderr || result.stdout;
        const output = stream.trim();

        return {
          content: [
            {
              type: "text",
              text: output === "" ? `bh ${args.join(" ")}: done` : output,
            },
          ],
          details: { args, code: result.code },
          isError: result.code !== 0,
        };
      },
    });
  };

  register(
    "bh_task",
    "Queue work in this repo through bulkhead.",
    "Queue work in this repo with bh_task.",
    Type.Object({ ask: Type.String({ description: "What to build" }) }),
    (params) => ["task", String(params.ask)],
  );

  register(
    "bh_plan",
    "Add a plan to this repo's inbox.",
    "Add a plan to this repo with bh_plan.",
    Type.Object({ path: Type.String({ description: "Path to the plan file" }) }),
    (params) => ["plan", String(params.path)],
  );

  register(
    "bh_spawn",
    "Spawn a crewmate for a backlog task.",
    "Spawn crew for a task with bh_spawn.",
    Type.Object({ task_id: Type.String({ description: "Task id" }) }),
    (params) => ["spawn", String(params.task_id)],
  );

  register(
    "bh_send",
    "Send a message to a crewmate.",
    "Message a crewmate with bh_send.",
    Type.Object({
      task_id: Type.String({ description: "Task id" }),
      message: Type.String({ description: "Message to deliver" }),
    }),
    (params) => ["send", String(params.task_id), String(params.message)],
  );

  register(
    "bh_status",
    "Read the status of this repo or one task.",
    "Read crew status with bh_status.",
    Type.Object({
      task_id: Type.Optional(Type.String({ description: "Task id" })),
    }),
    (params) =>
      params.task_id === undefined
        ? ["status"]
        : ["status", String(params.task_id)],
  );

  register(
    "bh_land",
    "Merge an approved task through the delivery gate.",
    "Land an approved task with bh_land.",
    Type.Object({ task_id: Type.String({ description: "Task id" }) }),
    (params) => ["land", String(params.task_id)],
  );

  register(
    "bh_teardown",
    "Tear a task down and return its worktree.",
    "Tear a task down with bh_teardown.",
    Type.Object({ task_id: Type.String({ description: "Task id" }) }),
    (params) => ["teardown", String(params.task_id)],
  );

  register(
    "bh_away",
    "Toggle away-mode for this repo.",
    "Toggle away-mode with bh_away.",
    Type.Object({ mode: StringEnum(["on", "off"] as const) }),
    (params) => ["away", String(params.mode)],
  );

  register(
    "bh_ls",
    "List registered repos and their tasks.",
    "List repos and tasks with bh_ls.",
    Type.Object({}),
    () => ["ls"],
  );
}
