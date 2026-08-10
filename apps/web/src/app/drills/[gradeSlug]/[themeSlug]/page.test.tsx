import { describe, expect, it, vi } from 'vitest';

import UnitPage, { dynamicParams, generateMetadata, generateStaticParams } from './page';

vi.mock('next/navigation', () => ({
  notFound: vi.fn(() => {
    throw new Error('NEXT_NOT_FOUND');
  }),
}));

describe('implemented unit route', () => {
  it('pre-renders and preselects every implemented unit from the registry', () => {
    expect(dynamicParams).toBe(false);
    expect(generateStaticParams()).toEqual([
      { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' },
      { gradeSlug: 'grade-1', themeSlug: 'one-digit-subtraction' },
      { gradeSlug: 'grade-2', themeSlug: 'two-digit-addition' },
      { gradeSlug: 'grade-2', themeSlug: 'multiplication-table' },
      { gradeSlug: 'grade-5', themeSlug: 'fraction-addition' },
      { gradeSlug: 'grade-5', themeSlug: 'fraction-subtraction' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-multiplication' },
      { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-1' },
      { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-2' },
      { gradeSlug: 'grade-7', themeSlug: 'linear-equation-1' },
      { gradeSlug: 'grade-7', themeSlug: 'linear-equation-2' },
    ]);

    const page = UnitPage({ params: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' } });
    expect(page.props.initialWebSettings).toEqual({
      schema_version: 3,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 3,
      seed: '',
    });
    expect(generateMetadata({ params: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition' } })).toMatchObject({
      title: '一桁の足し算 | AutoDrill',
    });

    const linear1 = UnitPage({ params: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-1' } });
    expect(linear1.props.initialWebSettings).toMatchObject({
      numeric_theme_id: 2,
      themeKey: 'jp.grade7.equation.linear.1',
    });
    expect(generateMetadata({ params: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-2' } })).toMatchObject({
      title: '一次方程式(2) | AutoDrill',
    });
  });

  it('404s Dummy and arbitrary unimplemented routes before metadata or UI discovery', () => {
    const dummy = { params: { gradeSlug: 'grade-2', themeSlug: 'Dummy1' } };
    expect(() => UnitPage(dummy)).toThrow('NEXT_NOT_FOUND');
    expect(() => generateMetadata(dummy)).toThrow('NEXT_NOT_FOUND');
    expect(() => UnitPage({ params: { gradeSlug: 'grade-1', themeSlug: 'missing' } })).toThrow('NEXT_NOT_FOUND');
  });
});
