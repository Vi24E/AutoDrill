import { describe, expect, it } from 'vitest';

import { problemSetIdFromSearch, urlWithProblemSetId, urlWithoutProblemSetId } from '@/domain/problem-set-url';

describe('problem-set URL', () => {
  it('reads the canonical seed query parameter without inventing a second identity format', () => {
    expect(problemSetIdFromSearch('?seed=7-1-5-Ab3Z-2')).toBe('7-1-5-Ab3Z-2');
    expect(problemSetIdFromSearch('?seed=%20%20')).toBeNull();
    expect(problemSetIdFromSearch('?other=value')).toBeNull();
  });

  it('replaces only the seed query parameter and preserves the current static route', () => {
    const href = 'https://example.test/AutoDrill/drills/grade-1/one-digit-addition?foo=bar&seed=old#x';
    expect(urlWithProblemSetId(href, '7-1-5-Ab3Z-2'))
      .toBe('/AutoDrill/drills/grade-1/one-digit-addition?foo=bar&seed=7-1-5-Ab3Z-2#x');
    expect(urlWithoutProblemSetId(href))
      .toBe('/AutoDrill/drills/grade-1/one-digit-addition?foo=bar#x');
  });
});
