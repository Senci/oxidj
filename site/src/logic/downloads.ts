const names = [
  ['oxidj-linux-x86_64.tar.gz', 'Linux x86-64'],
  ['oxidj-windows-x86_64.zip', 'Windows x86-64'],
  ['oxidj-macos-aarch64.tar.gz', 'macOS Apple Silicon'],
  ['SHA256SUMS', 'SHA256 checksums'],
] as const;

type Asset = { name: string; state: string; browser_download_url: string };
type Release = { draft: boolean; tag_name: string; assets: Asset[] };

export function releaseAssets(releases: Release[]): { label: string; url: string }[] {
  for (const release of releases) {
    if (
      release.draft ||
      !/^v\d+\.\d+\.\d+-alpha\.\d+$/.test(release.tag_name) ||
      !Array.isArray(release.assets)
    )
      continue;
    const result: { label: string; url: string }[] = [];
    for (const [name, label] of names) {
      const matches = release.assets.filter(
        (asset) => asset.name === name && asset.state === 'uploaded',
      );
      const expected = `https://github.com/Senci/oxidj/releases/download/${release.tag_name}/${name}`;
      if (matches.length !== 1 || matches[0]?.browser_download_url !== expected) break;
      result.push({ label, url: expected });
    }
    if (result.length === names.length) return result;
  }
  return [];
}
