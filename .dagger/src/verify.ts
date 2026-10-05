import { Container, File } from '@dagger.io/dagger';

export async function verifyRuntimeToolsImage(image: Container, desktopProbe: File): Promise<string> {
    return image.withMountedFile('/tmp/restless-desktop-probe.mjs', desktopProbe)
      .withUser('2000:2000').withEnvVariable('HOME', '/tmp').withExec(['/bin/sh', '-ec',
      'test ! -e /usr/local/bin/restless; test ! -e /usr/local/bin/restless-runtime-bridge; '
      + 'node --version; npm --version; codex --version; omp --version; pnpm --version; '
      + 'test ! -e /usr/bin/node; test -s /usr/share/novnc/vnc.html; test -s /usr/share/novnc/core/rfb.js; '
      + 'godot --headless --version; chromium --version; '
      + 'node -e \'if (typeof require("/usr/local/lib/node_modules/ws") !== "function") throw new Error("ws is unavailable")\'; '
      + 'test "$(id -u company)" = 2000; test "$(id -u effect)" = 2001; '
      + 'node /usr/local/lib/restless/browser-sbom.mjs > /tmp/restless-browser-inventory.json; '
      + 'cmp /usr/local/share/restless/browser.cdx.json /tmp/restless-browser-inventory.json; '
      + 'rm /tmp/restless-browser-inventory.json; '
      + 'printf "Browser inventory matches the executable version and SHA-256\\n"; '
      + 'test -s /opt/restless/godot/export_templates/4.7.2.stable/windows_release_x86_64.exe; '
      // Exercise the package manager's archive/install path after replacing its
      // compatible bundled dependencies, using only a local fixture as uid 2000.
      + 'mkdir -p /tmp/restless-package-check/source; '
      + 'printf \'{"name":"restless-runtime-tool-check","version":"1.0.0","main":"index.cjs","files":["index.{cjs,js}"]}\\n\' > /tmp/restless-package-check/source/package.json; '
      + 'printf \'module.exports = "archive-install-ok";\\n\' > /tmp/restless-package-check/source/index.cjs; '
      + 'cd /tmp/restless-package-check/source; npm pack --offline --ignore-scripts --pack-destination ..; '
      + 'npm install --offline --ignore-scripts --no-audit --no-fund --prefix /tmp/restless-package-check/consumer /tmp/restless-package-check/restless-runtime-tool-check-1.0.0.tgz; '
      + 'node -e \'if (require("/tmp/restless-package-check/consumer/node_modules/restless-runtime-tool-check") !== "archive-install-ok") throw new Error("npm archive installation failed")\'; '
      + 'node /tmp/restless-desktop-probe.mjs; '
      + 'printf "Runtime tool base is usable as the unprivileged company user without Core binaries\\n"',
    ], { useEntrypoint: false }).stdout();
}

export async function verifyCompanyRuntimeImage(image: Container, revision: string): Promise<string> {
    // A release assembles the image from the runtime layer rather than the Dockerfile's last stage;
    // both must start the same init with the same Runtime configuration.
    const entrypoint = await image.entrypoint();
    if (JSON.stringify(entrypoint) !== JSON.stringify(['/usr/local/bin/company-init'])) {
      throw new Error(`company Runtime entrypoint is ${JSON.stringify(entrypoint)}, not company-init`);
    }
    const checked = image.withEnvVariable('EXPECTED_REVISION', revision).withExec(['/bin/sh', '-ec',
      'test "$RESTLESS_COORDINATOR" = host.docker.internal:7791; test -s "$RESTLESS_CODEX_GPT6_SOL_CATALOG"; '
      + 'test -x /usr/local/bin/restless-scenario; test -x /usr/local/bin/restless-codex-runner; '
      + 'test "$(stat -c %a /usr/local/bin/company-init)" = 555; test -s /etc/supervisor/conf.d/company.conf; '
      + 'test "$(stat -c %a /etc/supervisor/conf.d/company.conf)" = 444; '
      + 'test "$(stat -c %u:%g /usr/local/bin)" = 0:0; test "$(stat -c %a /usr/local/bin)" = 755; '
      + 'test "$(stat -c %a /etc/supervisor/conf.d)" = 755; test "$(stat -c %a /opt/restless/skills)" = 555; '
      + 'restless --help; links="$(ldd /usr/local/bin/restless-runtime-bridge)"; '
      + 'printf "%s\\n" "$links"; ! printf "%s" "$links" | grep -q "not found"; '
      + 'test "$RESTLESS_SOURCE_REVISION" = "$EXPECTED_REVISION"; '
      + 'test -x /usr/local/bin/company-init; test -x /usr/local/bin/start-runtime-bridge; '
      + 'test -s /opt/restless/skills/scenario-evidence/SKILL.md; '
      + 'printf "Company Runtime binaries link and release metadata matches\\n"',
    ], { useEntrypoint: false });
    // Exercise the immutable launcher's actual uid/capability drop. The small
    // supervisor fixture avoids starting desktop services or a company daemon.
    return checked.withNewFile('/tmp/restless-privilege-probe',
      '#!/bin/sh\nset -eu\n/usr/local/bin/assert-runtime-privileges company\n'
      + 'printf passed > /tmp/restless-supervisor-probe/passed\n', { permissions: 0o555 })
      .withNewFile('/etc/supervisor/conf.d/company.conf',
        '[supervisord]\nnodaemon=true\nlogfile=/tmp/restless-supervisor-probe/supervisor.log\n'
        + 'logfile_maxbytes=0\npidfile=/tmp/restless-supervisor-probe/supervisor.pid\n'
        + 'childlogdir=/tmp/restless-supervisor-probe\n[program:privilege-probe]\n'
        + 'command=/tmp/restless-privilege-probe\nautostart=true\nautorestart=false\nstartsecs=0\n'
        + 'stdout_logfile=/tmp/restless-supervisor-probe/child.log\nstdout_logfile_maxbytes=0\nredirect_stderr=true\n')
      .withExec(['/bin/sh', '-ec',
        'mkdir -p /company/services/supervisor /tmp/restless-supervisor-probe; '
        + 'chown -R 2000:2000 /company /tmp/restless-supervisor-probe; '
        + '/usr/local/bin/start-company-supervisor > /tmp/restless-supervisor-probe/launcher.log 2>&1 & supervisor_pid=$!; '
        + 'trap \'kill -TERM "$supervisor_pid" 2>/dev/null || true; wait "$supervisor_pid" 2>/dev/null || true\' EXIT; '
        + 'for attempt in $(seq 1 100); do '
        + 'if test -f /tmp/restless-supervisor-probe/passed; then '
        + 'test "$(stat -c %u /tmp/restless-supervisor-probe/passed)" = 2000; '
        + 'printf "Company supervisor child has uid/gid 2000, no groups/capabilities, and NoNewPrivs\\n"; exit 0; fi; '
        + 'sleep 0.1; done; cat /tmp/restless-supervisor-probe/launcher.log; '
        + 'cat /tmp/restless-supervisor-probe/child.log 2>/dev/null || true; exit 1',
      ], { useEntrypoint: false }).stdout();
}

export async function verifyNativeDocumentsImage(image: Container): Promise<string> {
    return image.withExec(['node', '--input-type=module', '-e',
      'import assert from "node:assert/strict"; import http from "node:http"; import { generateKeyPair, exportJWK } from "jose"; import { NativeDocumentsCollaborationServer } from "./dist/server.js"; '
      + 'assert.equal(process.getuid(),10001); '
      + 'const {publicKey}=await generateKeyPair("EdDSA"); const key={...await exportJWK(publicKey),kid:"native-documents-000000000000000000000000",alg:"EdDSA",use:"sig"}; '
      + 'const jwks=http.createServer((req,res)=>{res.setHeader("content-type","application/json");res.end(JSON.stringify({keys:[key]}))}); '
      + 'await new Promise(resolve=>jwks.listen(17788,"127.0.0.1",resolve)); '
      + 'const store={initialize:async()=>{},close:async()=>{},ready:async()=>false,consumeSession:async()=>false}; '
      + 'const server=new NativeDocumentsCollaborationServer({address:"127.0.0.1",port:6688,companyId:"11111111-1111-4111-8111-111111111111",'
      + 'expectedIssuer:"https://plane.example.invalid",jwksUrl:new URL("http://127.0.0.1:17788/.well-known/restless-native-documents-jwks.json"),'
      + 'databaseUrl:"unused-with-store-fixture",debounceMs:10,maxDebounceMs:10},{store}); '
      + 'try{await server.listen(); const live=await fetch("http://127.0.0.1:6688/internal/v1/native-documents/live"); '
      + 'assert.equal(live.status,200); assert.equal((await live.json()).status,"live"); '
      + 'const ready=await fetch("http://127.0.0.1:6688/internal/v1/native-documents/ready"); assert.equal(ready.status,503); '
      + 'console.log("Native Documents image: uid 10001, HTTP liveness 200, missing dependencies correctly report readiness 503"); '
      + '}finally{await server.destroy(); await new Promise((resolve,reject)=>jwks.close(error=>error?reject(error):resolve()))}',
    ], { useEntrypoint: false }).stdout();
}

export async function verifyAccountPlaneImage(image: Container): Promise<string> {
    return image.withExec(['/bin/sh', '-ec',
      '/usr/local/bin/restlessd --help; /usr/local/bin/restless appliance --help; test -s /opt/restless/cockpit/index.html; '
      + 'node --input-type=module -e \'const { Model } = await import("/opt/restless/native-sheets/src/upstream.mjs"); '
      + 'if (typeof Model !== "function") throw new Error("Native Sheets engine is unavailable"); '
      + 'console.log("Account-plane binary links; bundled cockpit and native Sheets engine are usable")\'',
    ], { useEntrypoint: false }).stdout();
}
