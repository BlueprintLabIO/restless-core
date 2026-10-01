import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const packagePath = join(root, "node_modules/@oh-my-pi/pi-ai/package.json");
const patchPath = join(root, "patches/pi-ai-18.3.2-responses-namespace.patch");
const patch = readFileSync(patchPath);
const upgradePatch = readFileSync(join(root, "patches/pi-ai-18.3.2-responses-namespace-v1-upgrade.patch"));
const installed = JSON.parse(readFileSync(packagePath, "utf8"));
if (installed.version !== "18.3.2") {
	throw new Error(`Expected @oh-my-pi/pi-ai 18.3.2, found ${installed.version}`);
}

const apply = (body, ...args) => spawnSync("patch", ["--batch", "--fuzz=0", "-p1", "-d", root, ...args], {
	cwd: root,
	encoding: "utf8",
	input: body,
});
const alreadyApplied = apply(patch, "--dry-run", "--reverse", "--forward");
if (alreadyApplied.error) {
	throw new Error(`The system patch command is required for the pinned OMP gateway: ${alreadyApplied.error.message}`);
}
if (alreadyApplied.status === 0) {
	console.log("OMP Responses namespace patch is already applied");
} else {
	const check = apply(patch, "--dry-run", "--forward");
	const fromV1 = check.status !== 0;
	const selectedPatch = fromV1 ? upgradePatch : patch;
	if (fromV1) {
		const upgradeCheck = apply(upgradePatch, "--dry-run", "--forward");
		if (upgradeCheck.status !== 0) {
			throw new Error(`OMP patch does not match clean or prior patched pi-ai 18.3.2:\n${check.stdout}\n${upgradeCheck.stdout}\n${upgradeCheck.stderr || upgradeCheck.error}`);
		}
	}
	const result = apply(selectedPatch, "--forward");
	if (result.status !== 0) {
		throw new Error(`Failed to apply OMP Responses namespace patch:\n${result.stdout}\n${result.stderr || result.error}`);
	}
	console.log(`${fromV1 ? "Upgraded" : "Applied"} OMP Responses namespace patch for pinned pi-ai 18.3.2`);
}
