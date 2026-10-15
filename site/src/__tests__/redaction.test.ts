import { describe, expect, it } from 'vitest';

import { redact } from '../logic/redaction';

describe('redact', () => {
  it('returns an empty string for empty input', () => {
    expect(redact('')).toBe('');
  });

  it('masks the entire string when keep is omitted or 0', () => {
    expect(redact('secret')).toBe('██████');
    expect(redact('secret', 0)).toBe('██████');
  });

  it('keeps the leading characters visible and masks the rest', () => {
    expect(redact('secret', 2)).toBe('se████');
    expect(redact('hello', 3)).toBe('hel██');
  });

  it('keeps the entire string when keep equals the length', () => {
    expect(redact('hello', 5)).toBe('hello');
  });

  it('clamps keep to the string length when it exceeds it', () => {
    expect(redact('ab', 5)).toBe('ab');
  });

  it('clamps a negative keep to 0', () => {
    expect(redact('hello', -1)).toBe('█████');
  });

  it('preserves the total length of the input', () => {
    const input = 'a fairly long string to redact';
    expect(redact(input, 7)).toHaveLength(input.length);
  });

  it('handles single-character strings', () => {
    expect(redact('x')).toBe('█');
    expect(redact('x', 1)).toBe('x');
  });
});
