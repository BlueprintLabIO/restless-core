import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const packagePath = join(root, "node_modules/@oh-my-pi/pi-ai/package.json");
const patchPath = join(root, "patches/pi-ai-18.3.2-responses-namespace.patch");
const patch = readFileSync(patchPath);
const installed = JSON.parse(readFileSync(packagePath, "utf8"));
if (installed.version !== "18.3.2") {
	throw new Error(`Expected @oh-my-pi/pi-ai 18.3.2, found ${installed.version}`);
}

const apply = (...args) => spawnSync("patch", ["--batch", "--fuzz=0", "-p1", "-d", root, ...args], {
	cwd: root,
	encoding: "utf8",
	input: patch,
});
const alreadyApplied = apply("--dry-run", "--reverse", "--forward");
if (alreadyApplied.error) {
	throw new Error(`The system patch command is required for the pinned OMP gateway: ${alreadyApplied.error.message}`);
}
if (alreadyApplied.status === 0) {
	console.log("OMP Responses namespace patch is already applied");
} else {
	const check = apply("--dry-run", "--forward");
	if (check.status !== 0) {
		throw new Error(`OMP patch does not match pinned pi-ai 18.3.2:\n${check.stdout}\n${check.stderr || check.error}`);
	}
	const result = apply("--forward");
	if (result.status !== 0) {
		throw new Error(`Failed to apply OMP Responses namespace patch:\n${result.stdout}\n${result.stderr || result.error}`);
	}
	console.log("Applied OMP Responses namespace patch to pinned pi-ai 18.3.2");
}
