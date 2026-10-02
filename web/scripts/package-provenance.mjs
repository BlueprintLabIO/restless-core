import { execFileSync } from 'node:child_process';
import { chmodSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

// These source libraries contain no executable programs. Git does not preserve
// read/write bits, so filesystem modes must not change their release bytes.
export function normalizePackageModes(directory) {
	chmodSync(directory, 0o755);
	for (const entry of readdirSync(directory, { withFileTypes: true })) {
		const path = join(directory, entry.name);
		if (entry.isSymbolicLink()) throw new Error(`package cannot contain a symlink: ${entry.name}`);
		if (entry.isDirectory()) normalizePackageModes(path);
		else chmodSync(path, 0o644);
	}
}

// The release workflow proves its checkout SHA before supplying source-only
// Dagger inputs. Ordinary local packing reads the same metadata from Git.
export function packageProvenance(args, root, inputs) {
	const option = (name) => {
		const at = args.indexOf(`--${name}`);
		return at < 0 ? undefined : args[at + 1];
	};
	const supplied = option('source-revision');
	const epoch = option('source-date-epoch');
	if (args.includes('--qualification')) {
		if (supplied !== undefined || epoch !== undefined) throw new Error('qualification cannot claim release provenance');
		return { sourceRevision: null, sourceDirty: null, qualificationOnly: true, packedAt: new Date(0).toISOString() };
	}
	if (supplied !== undefined || epoch !== undefined) {
		if (!/^[0-9a-f]{40}$/.test(supplied ?? '') || !/^\d+$/.test(epoch ?? '')) {
			throw new Error('source-only release requires an exact revision and commit epoch');
		}
		return { sourceRevision: supplied, sourceDirty: false, packedAt: new Date(Number(epoch) * 1000).toISOString() };
	}
	const git = (...values) => execFileSync('git', values, { cwd: root, encoding: 'utf8' }).trim();
	return {
		sourceRevision: git('rev-parse', 'HEAD'),
		sourceDirty: git('status', '--porcelain', '--', ...inputs) !== '',
		packedAt: new Date(Number(git('show', '-s', '--format=%ct', 'HEAD')) * 1000).toISOString()
	};
}
