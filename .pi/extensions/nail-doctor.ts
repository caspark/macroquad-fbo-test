/**
 * Nail Doctor Extension
 *
 * Automatically runs `nail doctor` after any edit to docs/nails/*.md files.
 * If nail doctor reports errors (non-zero exit), sends a steering message
 * to the model with the error output and instructions to fix using the nail CLI.
 *
 * Project-local: installed by `nail setup` into .pi/extensions/ of projects that use nails.
 */

import * as path from "node:path";
import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	pi.on("tool_result", async (event, ctx) => {
		if (event.toolName !== "edit") return;
		if (event.isError) return;

		const filePath = (event.input as { path?: string }).path;
		if (!filePath) return;

		const resolved = path.isAbsolute(filePath) ? filePath : path.resolve(ctx.cwd, filePath);
		const relative = path.relative(ctx.cwd, resolved);

		if (!relative.startsWith("docs/nails/") || !relative.endsWith(".md")) return;

		const result = await pi.exec("nail", ["doctor"], { timeout: 10000 });

		if (result.code !== 0) {
			const errorOutput = (result.stdout + "\n" + result.stderr).trim();

			pi.sendMessage(
				{
					customType: "nail-doctor",
					content: [
						`\`nail doctor\` failed after editing \`${relative}\`:`,
						"",
						"```",
						errorOutput,
						"```",
						"",
						"Fix this nail integrity issue. Use the nails skill for reference on the nail CLI.",
						"Prefer using `nail update`, `nail reparent`, or other nail CLI commands to fix issues rather than editing files directly.",
						"If you must edit a file directly, ensure the format matches the nail data format described in the skill.",
					].join("\n"),
					display: true,
				},
				{ deliverAs: "steer", triggerTurn: true },
			);
		}
	});
}
