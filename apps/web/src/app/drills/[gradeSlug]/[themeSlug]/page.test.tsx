import { describe, expect, it, vi } from 'vitest';

import UnitPage, { dynamicParams, generateMetadata, generateStaticParams } from './page';
import { DRILL_SCHEMA_VERSION } from '@/domain/drill-engine';
import { IMPLEMENTED_THEMES } from '@/domain/curriculum';

vi.mock('next/navigation', () => ({
  notFound: vi.fn(() => {
    throw new Error('NEXT_NOT_FOUND');
  }),
}));

const params = (gradeSlug: string, themeSlug: string) => Promise.resolve({ gradeSlug, themeSlug });

describe('implemented unit route', () => {
  it('pre-renders and preselects every implemented unit from the registry', async () => {
    expect(dynamicParams).toBe(false);
    expect(generateStaticParams()).toEqual(IMPLEMENTED_THEMES.map((theme) => ({
      gradeSlug: theme.route.gradeSlug,
      themeSlug: theme.route.themeSlug,
    })));

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
