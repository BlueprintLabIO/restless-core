// v1 was published as AMD64+ARM64. v2 names the actually qualified platforms.
export function releasePlatforms(manifest) {
  if (manifest.manifest_version === 1) return ['linux/amd64', 'linux/arm64'];
  if (manifest.manifest_version !== 2) throw new Error('release manifest version is unsupported');
  const platforms = manifest.platforms;
  if (!Array.isArray(platforms) || platforms.length === 0
    || platforms.some((value) => !['linux/amd64', 'linux/arm64'].includes(value))
    || JSON.stringify(platforms) !== JSON.stringify([...new Set(platforms)].sort())) {
    throw new Error('v2 release platforms must be a nonempty sorted set of supported Linux platforms');
  }
  return platforms;
}
