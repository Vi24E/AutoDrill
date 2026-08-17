import { describe, expect, it, vi } from 'vitest';

import UnitPage, { dynamicParams, generateMetadata, generateStaticParams } from './page';
import { DRILL_SCHEMA_VERSION } from '@/domain/drill-engine';

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
      { gradeSlug: 'grade-2', themeSlug: 'column-addition-two-digit' },
      { gradeSlug: 'grade-2', themeSlug: 'column-subtraction-two-digit' },
      { gradeSlug: 'grade-3', themeSlug: 'column-addition-three-four-digit' },
      { gradeSlug: 'grade-3', themeSlug: 'column-subtraction-three-four-digit' },
      { gradeSlug: 'grade-2', themeSlug: 'multiplication-table' },
      { gradeSlug: 'grade-3', themeSlug: 'division-1' },
      { gradeSlug: 'grade-3', themeSlug: 'column-multiplication-one-digit' },
      { gradeSlug: 'grade-3', themeSlug: 'column-multiplication-two-digit' },
      { gradeSlug: 'grade-3', themeSlug: 'column-division-one-digit' },
      { gradeSlug: 'grade-4', themeSlug: 'column-division-two-digit' },
      { gradeSlug: 'grade-4', themeSlug: 'decimal-add-subtract' },
      { gradeSlug: 'grade-4', themeSlug: 'column-decimal-add-subtract' },
      { gradeSlug: 'grade-4', themeSlug: 'column-decimal-multiply-integer' },
      { gradeSlug: 'grade-4', themeSlug: 'column-decimal-divide-integer' },
      { gradeSlug: 'grade-5', themeSlug: 'fraction-addition' },
      { gradeSlug: 'grade-5', themeSlug: 'decimal-multiplication' },
      { gradeSlug: 'grade-5', themeSlug: 'decimal-division' },
      { gradeSlug: 'grade-5', themeSlug: 'column-decimal-multiplication' },
      { gradeSlug: 'grade-5', themeSlug: 'column-decimal-division' },
      { gradeSlug: 'grade-5', themeSlug: 'fraction-subtraction' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-multiplication' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-division' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-integer-multiplication' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-integer-division' },
      { gradeSlug: 'grade-6', themeSlug: 'fraction-summary-improper' },
      { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-1' },
      { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-2' },
      { gradeSlug: 'grade-7', themeSlug: 'linear-equation-1' },
      { gradeSlug: 'grade-7', themeSlug: 'linear-equation-2' },
      { gradeSlug: 'grade-8', themeSlug: 'simultaneous-equation-1' },
      { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-1' },
      { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-2' },
      { gradeSlug: 'grade-9', themeSlug: 'quadratic-equation-3' },
      { gradeSlug: 'bonus', themeSlug: 'liar-puzzle' },
    ]);

    const page = await UnitPage({ params: params('grade-1', 'one-digit-addition') });
    expect(page.props.initialWebSettings).toEqual({
      schema_version: DRILL_SCHEMA_VERSION,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 2,
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
    const simultaneous = await UnitPage({ params: params('grade-8', 'simultaneous-equation-1') });
    expect(simultaneous.props.initialWebSettings).toMatchObject({
      numeric_theme_id: 19,
      themeKey: 'jp.grade8.equation.simultaneous.1',
    });
  });

  it('404s arbitrary unimplemented routes before metadata or UI discovery', async () => {
    await expect(UnitPage({ params: params('grade-1', 'missing') })).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(generateMetadata({ params: params('grade-8', 'missing') })).rejects.toThrow('NEXT_NOT_FOUND');
  });
});
