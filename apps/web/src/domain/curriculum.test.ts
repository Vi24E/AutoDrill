import { describe, expect, it } from 'vitest';

import {
  ADDITION_AND_SUBTRACTION_GENRE,
  CURRICULUM_TREE,
  DEFAULT_WEB_DRILL_SETTINGS,
  IMPLEMENTED_THEMES,
  ONE_DIGIT_ADDITION_THEME,
  RECOMMENDED_GENRES,
  findImplementedThemeByRoute,
} from '@/domain/curriculum';

describe('Web curriculum registry', () => {
  it('exposes the stable implemented-unit boundary for later engine integration', () => {
    expect(ONE_DIGIT_ADDITION_THEME).toMatchObject({
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      generator_revision: 2,
      problemCount: 20,
      layout: { problem_count: 20, columns: 2, rows: 10 },
      route: {
        gradeSlug: 'grade-1',
        themeSlug: 'one-digit-addition',
        pathname: '/drills/grade-1/one-digit-addition',
      },
    });
    expect(DEFAULT_WEB_DRILL_SETTINGS).toEqual({
      schema_version: 2,
      numeric_theme_id: 1,
      themeKey: 'jp.grade1.addition.one_digit',
      difficulty: 3,
      seed: '',
    });
  });

  it('derives recommended from the canonical grade tree by object identity', () => {
    expect(RECOMMENDED_GENRES[0]).toBe(ADDITION_AND_SUBTRACTION_GENRE);
    expect(RECOMMENDED_GENRES[0]).toBe(CURRICULUM_TREE[0]?.genres[0]);
    expect(RECOMMENDED_GENRES[0]?.themes[0]).toBe(ONE_DIGIT_ADDITION_THEME);
    expect(RECOMMENDED_GENRES[0]?.themes[0]).toBe(CURRICULUM_TREE[0]?.genres[0]?.themes[0]);
  });

  it('maps grade-1 through grade-9 and keeps Dummy themes unimplemented', () => {
    expect(CURRICULUM_TREE.map((grade) => grade.slug)).toEqual([
      'grade-1', 'grade-2', 'grade-3', 'grade-4', 'grade-5',
      'grade-6', 'grade-7', 'grade-8', 'grade-9',
    ]);
    expect(CURRICULUM_TREE.map((grade) => grade.label)).toEqual([
      '小学1年生', '小学2年生', '小学3年生', '小学4年生', '小学5年生',
      '小学6年生', '中学1年生', '中学2年生', '中学3年生',
    ]);
    const dummyThemes = CURRICULUM_TREE.flatMap((grade) => grade.genres)
      .flatMap((genre) => genre.themes)
      .filter((theme) => !theme.implemented);
    expect(dummyThemes).toHaveLength(9);
    expect(dummyThemes.every((theme) => theme.route === null && theme.search === null)).toBe(true);
  });

  it('resolves only implemented public routes', () => {
    expect(IMPLEMENTED_THEMES).toEqual([ONE_DIGIT_ADDITION_THEME]);
    expect(findImplementedThemeByRoute('grade-1', 'one-digit-addition')).toBe(ONE_DIGIT_ADDITION_THEME);
    expect(findImplementedThemeByRoute('grade-2', 'Dummy1')).toBeUndefined();
    expect(findImplementedThemeByRoute('grade-1', 'missing')).toBeUndefined();
  });
});
