import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('release downloads', () => {
  it('offers a direct Download link to the GitHub releases listing without JavaScript', () => {
    const html = readFileSync('index.html', 'utf8');
    expect(html).toMatch(
      /<a\s+class="button"\s+href="https:\/\/github\.com\/Senci\/oxidj\/releases">\s*Download\s*<\/a>/,
    );
  });
});
