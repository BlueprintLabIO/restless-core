// Complete the non-secret setup prepared by Company → Members.
// Preparation writes reviewable files only. Activation is a separate appliance command.
import { cp, open, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { fileURLToPath, pathToFileURL } from "node:url";
import { configure } from "./configure.mjs";
import { origin, sameSite } from "../src/config.mjs";

export function validatePlan(plan) {
  if (
    plan.version !== 1 ||
    !isAbsolute(plan.core_home) ||
    !["private", "https"].includes(plan.access) ||
    !Array.isArray(plan.companies) ||
    !plan.companies.length ||
    plan.companies.length > 100
  )
    throw Error("Prepare a new sharing setup from Company → Members");
  const account = origin(plan.account_origin),
    core = origin(plan.core_origin);
  if (
    account.origin === core.origin ||
    account.protocol !== core.protocol ||
    !sameSite(account, core)
  )
    throw Error("Accounts and Core need separate addresses on the same site");
  if (
    typeof plan.owner_email !== "string" ||
    !/^\S+@\S+\.\S+$/.test(plan.owner_email) ||
    plan.owner_email.length > 254
  )
    throw Error("The owner's verified email is required");
  const handles = new Set(),
    ids = new Set(),
    cells = new Set();
  const uuid =
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
  for (const company of plan.companies) {
    if (
      !/^[a-z][a-z0-9_]{0,62}$/.test(company.id) ||
      !uuid.test(company.company_id) ||
      !uuid.test(company.cell_id) ||
      typeof company.name !== "string" ||
      !company.name.trim() ||
      company.name.length > 120 ||
      handles.has(company.id) ||
      ids.has(company.company_id) ||
      cells.has(company.cell_id)
    )
      throw Error(
        "The sharing setup has invalid or repeated company coordinates",
      );
    handles.add(company.id);
    ids.add(company.company_id);
    cells.add(company.cell_id);
  }
  if (!handles.has(plan.company))
    throw Error("The selected company is missing from this setup");
  return plan;
}

export async function setup(options) {
  const file = await open(options.plan, "r");
  let input;
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.size > 65536)
      throw Error("Sharing setup must be a regular file within its size limit");
    const bytes = Buffer.alloc(65537);
    const { bytesRead } = await file.read(bytes);
    if (bytesRead > 65536) throw Error("Sharing setup exceeds its size limit");
    input = bytes.subarray(0, bytesRead).toString("utf8");
  } finally {
    await file.close();
  }
  const plan = validatePlan(JSON.parse(input));
  const corePort = Number(options.corePort ?? 7788);
  if (!Number.isInteger(corePort) || corePort < 1 || corePort > 65535)
    throw Error("Core's local port must be an integer from 1 to 65535");
  const current = (await readdir(join(plan.core_home, "companies")))
    .filter((name) => name.endsWith(".toml"))
    .map((name) => name.slice(0, -5))
    .sort();
  const planned = plan.companies.map((company) => company.id).sort();
  if (JSON.stringify(current) !== JSON.stringify(planned))
    throw Error(
      "The companies on this host changed. Prepare sharing setup again from Members.",
    );
  const company = plan.companies.find((company) => company.id === plan.company);
  const result = await configure({
    coreHome: plan.core_home,
    company: plan.company,
    companyName: company.name,
    companies: plan.companies,
    companyImage: options.companyImage ?? plan.company_image,
    origin: plan.account_origin,
    coreOrigin: plan.core_origin,
    ownerEmail: plan.owner_email,
    databaseUrlFile: options.databaseUrlFile,
    smtpFile: options.smtpFile,
    output: options.output,
    port: options.port,
  });
  try {
    // Keep the account host independent of the development checkout. Only its
    // production source and locked dependencies belong in the installation.
    const source = resolve(dirname(fileURLToPath(import.meta.url)), "..");
    const service = join(options.output, "service");
    for (const name of ["src", "public", "package.json", "package-lock.json"])
      await cp(join(source, name), join(service, name), {
        recursive: true,
        errorOnExist: true,
        force: false,
      });
    const config = JSON.parse(await readFile(result.configPath, "utf8"));
    const routes = [
      `${plan.account_origin} {\n  reverse_proxy 127.0.0.1:${config.port} {\n    header_up Host ${new URL(plan.account_origin).host}\n  }\n}`,
      `${plan.core_origin} {\n  reverse_proxy 127.0.0.1:${corePort} {\n    header_up Host ${new URL(plan.core_origin).host}\n  }\n}`,
    ];
    await writeFile(join(options.output, "Caddyfile"), routes[0] + "\n", {
      flag: "wx",
      mode: 0o600,
    });
    await writeFile(
      join(options.output, "Caddyfile.shared"),
      routes.join("\n\n") + "\n",
      {
        flag: "wx",
        mode: 0o600,
      },
    );
    const quote = (value) => "'" + String(value).replaceAll("'", "'\\''") + "'";
    const instructions = `# Sharing setup\n\nThis setup preserves all ${plan.companies.length} existing companies and their immutable identities.\n\n1. Install the prepared account host's locked production dependencies with Node 24+, then run it under your normal process supervisor:\n\n   cd ${quote(service)}\n   npm ci --omit=dev --ignore-scripts\n   RESTLESS_IDENTITY_CONFIG=${quote(result.configPath)} npm start\n\n2. Prepare HTTPS routing for both configured addresses. Caddyfile initially routes only the account address. Caddyfile.shared adds the company route after activation. These are suitable for a Caddy installation you operate. Private-network addresses need certificates trusted by the team's browsers. Keep both backend listeners private. Make only the account address reachable until step 3 succeeds; the local-owner listener must never be exposed.\n\n3. Once the account service's HTTPS address is reachable, activate authenticated entry with the installed appliance CLI:\n\n   restless appliance enable-sharing --environment ${quote(result.coreEnvironment)}\n\n   The appliance verifies the new entry configuration, drains active work, restarts and restores the previous settings if activation fails. Existing credentials and company data remain in place. Then apply Caddyfile.shared to make the company address reachable through its HTTPS routing.\n\n4. Open ${plan.account_origin}, create and verify ${plan.owner_email}, and set up access to the companies you own. Invite colleagues from Company → Members. Each person signs in with their own verified address.\n\n${new URL(plan.core_origin).hostname.endsWith(".ts.net") ? `These are Tailscale addresses. Skip the Caddy files: after step 1, run\n\n   restless appliance enable-sharing --tailscale --environment ${quote(result.coreEnvironment)}\n\nwhich publishes both routes with tailscale serve around the same activation and checks them over the tailnet.\n\n` : "Tailscale is the recommended private transport: restless appliance tailscale-addresses prints the addresses to prepare.\n\n"}Without Tailscale, SSH is optional transport. Use an ordinary OpenSSH SOCKS tunnel (ssh -N -D 127.0.0.1:1080 user@host) and a browser configured to use that SOCKS proxy with proxy DNS. Open the configured HTTPS addresses through it. SSH does not replace company sign-in. Do not forward the unauthenticated local-owner port.\n`;
    await writeFile(join(options.output, "SETUP.md"), instructions, {
      flag: "wx",
      mode: 0o600,
    });
    return result;
  } catch (error) {
    // configure() created this exact output exclusively; a partial preparation
    // can be removed without touching any previous installation.
    await rm(options.output, { recursive: true, force: true });
    throw error;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    const { values } = parseArgs({
      options: Object.fromEntries(
        [
          "plan",
          "database-url-file",
          "smtp-file",
          "output",
          "company-image",
          "port",
          "core-port",
        ].map((name) => [name, { type: "string" }]),
      ),
    });
    for (const name of ["plan", "database-url-file", "smtp-file", "output"])
      if (!values[name]) throw Error(`Missing --${name}`);
    const result = await setup(
      Object.fromEntries(
        Object.entries(values).map(([name, value]) => [
          name.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase()),
          value,
        ]),
      ),
    );
    console.log(
      `Sharing files prepared. Review ${join(values.output, "SETUP.md")} before activating.\nPrivate account configuration: ${result.configPath}`,
    );
  } catch (error) {
    console.error(`Sharing setup was not activated: ${error.message}`);
    process.exitCode = 1;
  }
}
