import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const version = "18.8.6";
const root = dirname(fileURLToPath(import.meta.url));
const packagePath = join(root, "node_modules/@oh-my-pi/pi-ai/package.json");
const patches = ["responses-namespace", "pi-wire-efficiency"];
const installed = JSON.parse(readFileSync(packagePath, "utf8"));
if (installed.version !== version) {
	throw new Error(`Expected @oh-my-pi/pi-ai ${version}, found ${installed.version}`);
}

for (const name of patches) {
	const patch = readFileSync(join(root, `patches/pi-ai-${version}-${name}.patch`));
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
		console.log(`OMP ${name} patch is already applied`);
	} else {
		const result = apply("--forward");
		if (result.status !== 0) {
			throw new Error(`Failed to apply OMP ${name} patch:\n${result.stdout}\n${result.stderr || result.error}`);
		}
		console.log(`Applied OMP ${name} patch for pinned pi-ai ${version}`);
	}
}
