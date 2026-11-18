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
 *   - `pi.sendUserMessage(text, { deliverAs: "followUp" })` to deliver wakes.
 *   - `pi.on("session_shutdown")` to close the socket.
 *
 * The wake protocol is version 1.x; both sides refuse another major. The
turn-end guard arrives in a later unit.
 */
import { connect, type Socket } from "node:net";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { StringEnum } from "@earendil-works/pi-ai";
import { Type, type TSchema } from "typebox";

const BH_BIN = process.env.BULKHEAD_BH_BIN ?? "bh";
const PROTOCOL_MAJOR = 1;
const PROTOCOL_MINOR = 0;
const RECONNECT_MS = 2000;

export default function bulkheadOrch(pi: ExtensionAPI): void {
  registerBhTools(pi);
  startWakeClient(pi);
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
    "Add a plan to the inbox, or triage one by id.",
    "Add or triage plans with bh_plan.",
    Type.Object({
      path: Type.Optional(Type.String({ description: "Plan file to add" })),
      accept: Type.Optional(Type.String({ description: "Plan id to accept" })),
      decline: Type.Optional(Type.String({ description: "Plan id to decline" })),
      supersede: Type.Optional(Type.String({ description: "Plan id to supersede" })),
      reason: Type.Optional(Type.String({ description: "Reason for decline or supersede" })),
    }),
    (params) => {
      if (typeof params.path === "string") {
        return ["plan", params.path];
      }
      if (typeof params.accept === "string") {
        return ["plan", "--accept", params.accept];
      }
      if (typeof params.decline === "string") {
        const args = ["plan", "--decline", params.decline];
        if (typeof params.reason === "string") {
          args.push("--reason", params.reason);
        }
        return args;
      }
      if (typeof params.supersede === "string") {
        const args = ["plan", "--supersede", params.supersede];
        if (typeof params.reason === "string") {
          args.push("--reason", params.reason);
        }
        return args;
      }
      throw new Error("bh_plan needs a path or an accept/decline/supersede id");
    },
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

interface Wake {
  id: string;
  repo: string;
  task?: string | null;
  kind: string;
  summary: string;
  created_at: number;
}

interface Envelope {
  version: { major: number; minor: number };
  message: { type: string; [key: string]: unknown };
}

/**
 * Connects to bhd, delivers wakes as follow-up messages, and acknowledges them.
 *
 * Wakes are deduped on id, so an at-least-once redelivery is delivered once.
 * A missing socket is not an error: the extension simply runs without a daemon.
 */
function startWakeClient(pi: ExtensionAPI): void {
  const socketPath = process.env.BULKHEAD_SOCKET;
  const repo = process.env.BULKHEAD_REPO;
  if (!socketPath || !repo) {
    return;
  }

  const delivered = new Set<string>();
  let socket: Socket | undefined;
  let buffer = "";
  let closed = false;
  let activeTurn = false;
  const deferred: Wake[] = [];

  const send = (message: Record<string, unknown>): void => {
    if (!socket || socket.destroyed) {
      return;
    }
    socket.write(
      JSON.stringify({
        version: { major: PROTOCOL_MAJOR, minor: PROTOCOL_MINOR },
        message,
      }) + "\n",
    );
  };

  const flushDeferred = (): void => {
    const wakes = deferred.splice(0, deferred.length);
    if (wakes.length === 0) {
      return;
    }
    const text = wakes.map((wake) => `- ${wake.summary}`).join("\n");
    try {
      pi.sendUserMessage(
        `bulkhead: ${wakes.length} wake(s) arrived during your turn:\n${text}`,
        { deliverAs: "followUp" },
      );
    } catch {
      deferred.push(...wakes);
      return;
    }
    for (const wake of wakes) {
      send({ type: "ack", wake: wake.id });
    }
  };

  const handle = (envelope: Envelope): void => {
    if (envelope.version.major !== PROTOCOL_MAJOR) {
      return;
    }
    if (envelope.message.type !== "wake") {
      return;
    }

    const wake = envelope.message.wake as Wake | undefined;
    if (!wake) {
      return;
    }
    if (delivered.has(wake.id)) {
      send({ type: "ack", wake: wake.id });
      return;
    }

    delivered.add(wake.id);

    // While a turn is running, hold the wake until it settles so the guard can
    // deliver it as one follow-up and only then acknowledge it (at-least-once).
    if (activeTurn) {
      deferred.push(wake);
      return;
    }

    try {
      pi.sendUserMessage(wake.summary, { deliverAs: "followUp" });
    } catch {
      delivered.delete(wake.id);
      return;
    }
    send({ type: "ack", wake: wake.id });
  };

  const connectOnce = (): void => {
    if (closed) {
      return;
    }
    socket = connect(socketPath);
    socket.setEncoding("utf8");
    socket.on("connect", () => send({ type: "hello", repo, pid: process.pid }));
    socket.on("data", (chunk: string) => {
      buffer += chunk;
      let index = buffer.indexOf("\n");
      while (index >= 0) {
        const line = buffer.slice(0, index);
        buffer = buffer.slice(index + 1);
        index = buffer.indexOf("\n");
        if (line.trim() === "") {
          continue;
        }
        try {
          handle(JSON.parse(line) as Envelope);
        } catch {
          // Ignore a malformed frame; bhd redelivers on the next batch.
        }
      }
    });
    socket.on("error", () => {
      // The close handler schedules a reconnect.
    });
    socket.on("close", () => {
      if (!closed) {
        setTimeout(connectOnce, RECONNECT_MS);
      }
    });
  };

  connectOnce();

  // Turn-end guard: if wakes arrived while a turn ran, deliver them now.
  pi.on("agent_start", async () => {
    activeTurn = true;
  });
  pi.on("agent_settled", async () => {
    activeTurn = false;
    flushDeferred();
  });

  pi.on("session_shutdown", async () => {
    closed = true;
    socket?.destroy();
  });
}
