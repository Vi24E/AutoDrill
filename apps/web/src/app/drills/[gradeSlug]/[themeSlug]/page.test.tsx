import { describe, expect, it, vi } from 'vitest';

import UnitPage, { dynamicParams, generateMetadata, generateStaticParams } from './page';

vi.mock('next/navigation', () => ({
  notFound: vi.fn(() => {
    throw new Error('NEXT_NOT_FOUND');
  }),
}));

const params = (gradeSlug: string, themeSlug: string) => Promise.resolve({ gradeSlug, themeSlug });

describe('implemented unit route', () => {
  it('pre-renders and preselects every implemented unit from the registry', async () => {
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

    const page = await UnitPage({ params: params('grade-1', 'one-digit-addition') });
    expect(page.props.initialWebSettings).toEqual({
      schema_version: 3,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 3,
      seed: '',
    });
    await expect(generateMetadata({ params: params('grade-1', 'one-digit-addition') })).resolves.toMatchObject({
      title: '一桁の足し算 | AutoDrill',
    });

    const linear1 = await UnitPage({ params: params('grade-7', 'linear-equation-1') });
    expect(linear1.props.initialWebSettings).toMatchObject({
      numeric_theme_id: 2,
      themeKey: 'jp.grade7.equation.linear.1',
    });
    await expect(generateMetadata({ params: params('grade-7', 'linear-equation-2') })).resolves.toMatchObject({
      title: '一次方程式(2) | AutoDrill',
    });
  });

  it('404s Dummy and arbitrary unimplemented routes before metadata or UI discovery', async () => {
    const dummy = { params: params('grade-2', 'Dummy1') };
    await expect(UnitPage(dummy)).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(generateMetadata(dummy)).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(UnitPage({ params: params('grade-1', 'missing') })).rejects.toThrow('NEXT_NOT_FOUND');
  });
});
