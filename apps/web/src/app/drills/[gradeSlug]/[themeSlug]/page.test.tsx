import { describe, expect, it, vi } from 'vitest';

import UnitPage, { dynamicParams, generateMetadata, generateStaticParams } from './page';

vi.mock('next/navigation', () => ({
  notFound: vi.fn(() => {
    throw new Error('NEXT_NOT_FOUND');
  }),
}));

describe('implemented unit route', () => {
  it('pre-renders and preselects only the implemented unit', () => {
    expect(dynamicParams).toBe(false);
    expect(generateStaticParams()).toEqual([
      { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' },
    ]);

    const page = UnitPage({ params: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' } });
    expect(page.props.initialWebSettings).toEqual({
      schema_version: 2,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 3,
      seed: '',
    });
    expect(generateMetadata({ params: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' } })).toMatchObject({
      title: '一桁の足し算 | AutoDrill',
    });
  });

  it('404s Dummy and arbitrary unimplemented routes before metadata or UI discovery', () => {
    const dummy = { params: { gradeSlug: 'grade-2', themeSlug: 'Dummy1' } };
    expect(() => UnitPage(dummy)).toThrow('NEXT_NOT_FOUND');
    expect(() => generateMetadata(dummy)).toThrow('NEXT_NOT_FOUND');
    expect(() => UnitPage({ params: { gradeSlug: 'grade-1', themeSlug: 'missing' } })).toThrow('NEXT_NOT_FOUND');
  });
});
