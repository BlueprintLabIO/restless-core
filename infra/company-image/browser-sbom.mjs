import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { createReadStream, realpathSync } from 'node:fs';

const binary = realpathSync(process.argv[2] ?? '/usr/bin/chromium');
const output = execFileSync(binary, ['--version'], { encoding: 'utf8' }).trim();
const version = output.match(/^Google Chrome(?: for Testing)? (\d+\.\d+\.\d+\.\d+)$/)?.[1];
if (!version) throw new Error(`Unrecognized browser version: ${output}`);
const hash = createHash('sha256');
for await (const chunk of createReadStream(binary)) hash.update(chunk);
const checksum = hash.digest('hex');

// CycloneDX records this actual executable even when a scanner's binary-string
// classifier does not recognize its layout. It supplements the OS inventory.
console.log(JSON.stringify({
  bomFormat: 'CycloneDX',
  specVersion: '1.6',
  version: 1,
  components: [{
    type: 'application',
    'bom-ref': `chrome-sha256-${checksum}`,
    name: 'chrome',
    version,
    publisher: 'Google',
    cpe: `cpe:2.3:a:google:chrome:${version}:*:*:*:*:*:*:*`,
    purl: `pkg:generic/chrome@${version}`,
    hashes: [{ alg: 'SHA-256', content: checksum }],
    properties: [{ name: 'restless:executable:path', value: binary }],
  }],
}, null, 2));
