import { access, readdir, readFile } from 'node:fs/promises';
import { isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const journalDirectory = resolve(root, 'src/content/journal');
const expectedExperiments = ['EXP-01', 'EXP-02', 'EXP-03', 'EXP-04', 'EXP-05', 'EXP-06', 'EXP-07'];
const failures = [];
const locatorsOnly = process.argv.includes('--locators-only');
const repositoryRoot = resolve(root, '..');

async function exists(path) {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

const articleFiles = (await readdir(journalDirectory)).filter((file) => file.endsWith('.md'));
const articleIds = new Set(articleFiles.map((file) => file.slice(0, -3)));
const citedExperiments = new Set();

for (const file of articleFiles) {
  const content = await readFile(resolve(journalDirectory, file), 'utf8');
  const frontMatter = content.match(/^---\n([\s\S]*?)\n---/);

  if (frontMatter == null) {
    failures.push(`${file}: missing front matter`);
    continue;
  }

  for (const match of frontMatter[1].matchAll(/EXP-\d{2}/g)) citedExperiments.add(match[0]);

  const locators = [...frontMatter[1].matchAll(/^\s+locator:\s+"([^"]+)"$/gm)].map((match) => match[1]);
  if (locators.length === 0) failures.push(`${file}: no evidence locator`);

  for (const locator of locators) {
    const source = resolve(repositoryRoot, locator);
    const sourceRelativePath = relative(repositoryRoot, source);
    const sourceEscapesRepository = sourceRelativePath.startsWith('..') || isAbsolute(sourceRelativePath);

    if (sourceEscapesRepository) {
      failures.push(`${file}: evidence locator escapes the repository ${locator}`);
    } else if (!locatorsOnly && !(await exists(source))) {
      failures.push(`${file}: missing evidence source ${locator}`);
    }
  }
}

const corpus = await readFile(resolve(root, 'src/data/corpus.ts'), 'utf8');
const corpusRecords = [...corpus.matchAll(/experiment:\s+'(EXP-\d{2})',[\s\S]*?state:\s+'(Published|Deferred)',[\s\S]*?home:\s+'([^']+)'/g)];

for (const experiment of expectedExperiments) {
  const matches = corpusRecords.filter((match) => match[1] === experiment);
  if (matches.length !== 1) {
    failures.push(`${experiment}: expected exactly one corpus record, found ${matches.length}`);
    continue;
  }

  const [, , state, home] = matches[0];
  if (state === 'Published') {
    const journalMatch = home.match(/^\/journal\/([^/]+)\/$/);
    if (journalMatch == null || !articleIds.has(journalMatch[1])) {
      failures.push(`${experiment}: published home ${home} has no journal article`);
    }
  } else if (home !== '/research/corpus/') {
    failures.push(`${experiment}: deferred home must be /research/corpus/`);
  }
}

for (const experiment of expectedExperiments.slice(0, -1)) {
  if (!citedExperiments.has(experiment)) failures.push(`${experiment}: no journal evidence reference`);
}

if (failures.length > 0) {
  console.error('Corpus check rejected the site:\n' + failures.map((failure) => `  ${failure}`).join('\n'));
  process.exit(1);
}

const verificationScope = locatorsOnly ? 'are internally consistent' : 'are traceable to source files';
console.log(`Corpus check passed: ${articleFiles.length} articles and ${expectedExperiments.length} experiment records ${verificationScope}.`);
