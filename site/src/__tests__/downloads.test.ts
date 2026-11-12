import { describe, expect, it } from 'vitest';
import { releaseAssets } from '../logic/downloads';

const tag = 'v0.1.0-alpha.1';
const asset = (name: string, releaseTag = tag) => ({
  name,
  state: 'uploaded',
  browser_download_url: `https://github.com/Senci/oxidj/releases/download/${releaseTag}/${name}`,
});
const names = [
  'oxidj-linux-x86_64.tar.gz',
  'oxidj-windows-x86_64.zip',
  'oxidj-macos-aarch64.tar.gz',
  'SHA256SUMS',
];
const complete = { draft: false, tag_name: tag, assets: names.map((name) => asset(name)) };

describe('release downloads', () => {
  it('offers only a complete release with all archives and checksums bound to its tag', () => {
    expect(releaseAssets([complete]).map((item) => item.label)).toEqual([
      'Linux x86-64',
      'Windows x86-64',
      'macOS Apple Silicon',
      'SHA256 checksums',
    ]);
    expect(releaseAssets([{ ...complete, assets: complete.assets.slice(0, 3) }])).toEqual([]);
    expect(
      releaseAssets([
        {
          ...complete,
          assets: [...complete.assets.slice(0, 3), asset('SHA256SUMS', 'v0.1.0-alpha.2')],
        },
      ]),
    ).toEqual([]);
    expect(releaseAssets([{ ...complete, draft: true }])).toEqual([]);
  });
  it('falls back to an older complete release when the newest is incomplete', () => {
    expect(
      releaseAssets([{ ...complete, tag_name: 'v0.1.0-alpha.2', assets: [] }, complete]),
    ).toHaveLength(4);
  });
});
