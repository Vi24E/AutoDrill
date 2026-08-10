import { describe, expect, it } from 'vitest';

import {
  CURRICULUM_TREE,
  DEFAULT_WEB_DRILL_SETTINGS,
  IMPLEMENTED_THEMES,
  LINEAR_EQUATION_1_THEME,
  LINEAR_EQUATION_2_THEME,
  ONE_DIGIT_ADDITION_THEME,
  RECOMMENDED_GENRES,
  findImplementedThemeByRoute,
} from '@/domain/curriculum';
import { ALL_MATH_STRUCTURES } from '@/domain/theme-registry';

describe('Web curriculum registry', () => {
  it('registers all implemented arithmetic and equation themes from one data model', () => {
    expect(IMPLEMENTED_THEMES.map((theme) => theme.numeric_theme_id)).toEqual([1, 4, 5, 6, 9, 11, 10, 7, 8, 2, 3]);
    expect(ONE_DIGIT_ADDITION_THEME).toMatchObject({
      numeric_theme_id: 1,
      generator_revision: 3,
      recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
      problemCount: 20,
      layout: { problem_count: 20, columns: 2, rows: 10 },
    });
    for (const theme of [LINEAR_EQUATION_1_THEME, LINEAR_EQUATION_2_THEME]) {
      expect(theme).toMatchObject({
        grade: { slug: 'grade-7', label: '中学1年生' },
        gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' },
        recommendedGenre: { genreKey: 'equation', label: '方程式' },
        problemCount: 16,
        layout: { problem_count: 16, columns: 2, rows: 8 },
        promptKind: 'linear_equation',
        worksheet: { answerPrefix: 'x =' },
      });
      expect(theme.inputInterface).toEqual({
        type: 'structured_math',
        allowed_structures: ALL_MATH_STRUCTURES,
      });
    }
  });

  it('exposes exactly the five requested Recommended sections and defaults to addition', () => {
    expect(DEFAULT_WEB_DRILL_SETTINGS).toEqual({
      schema_version: 3,
      numeric_theme_id: ONE_DIGIT_ADDITION_THEME.numeric_theme_id,
      themeKey: ONE_DIGIT_ADDITION_THEME.themeKey,
      difficulty: 3,
      seed: '',
    });
    expect(RECOMMENDED_GENRES.map(({ genreKey, label }) => ({ genreKey, label }))).toEqual([
      { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
      { genreKey: 'multiplication-and-division', label: '掛け算と割り算' },
      { genreKey: 'fractions', label: '分数' },
      { genreKey: 'negative-numbers', label: '負の数' },
      { genreKey: 'equation', label: '方程式' },
    ]);
    expect(RECOMMENDED_GENRES.flatMap((genre) => genre.themes)).toEqual(IMPLEMENTED_THEMES);
  });

  it('never exposes negative input capability for elementary-school themes', () => {
    const elementary = IMPLEMENTED_THEMES.filter((theme) => Number(theme.grade.slug.slice(6)) <= 6);
    expect(elementary.length).toBeGreaterThan(0);
    for (const theme of elementary) {
      if (theme.inputInterface.type === 'simple_numeric') {
        expect(theme.inputInterface.allow_negative).toBe(false);
      } else {
        expect(theme.inputInterface.allowed_structures).not.toContain('negative');
        expect(theme.inputInterface.allowed_structures).not.toContain('plus_minus');
      }
    }
  });

  it('maps grade-1 through grade-9 and places equations under 中1 / 一次方程式', () => {
    expect(CURRICULUM_TREE.map((grade) => grade.slug)).toEqual([
      'grade-1', 'grade-2', 'grade-3', 'grade-4', 'grade-5',
      'grade-6', 'grade-7', 'grade-8', 'grade-9',
    ]);
    expect(CURRICULUM_TREE.map((grade) => grade.label)).toEqual([
      '小学1年生', '小学2年生', '小学3年生', '小学4年生', '小学5年生',
      '小学6年生', '中学1年生', '中学2年生', '中学3年生',
    ]);
    const grade7 = CURRICULUM_TREE[6]!;
    const equations = grade7.genres.find((genre) => genre.genreKey === 'linear-equation')!;
    expect(equations.label).toBe('一次方程式');
    expect(equations.themes).toEqual([LINEAR_EQUATION_1_THEME, LINEAR_EQUATION_2_THEME]);

    const dummyThemes = CURRICULUM_TREE.flatMap((grade) => grade.genres)
      .flatMap((genre) => genre.themes)
      .filter((theme) => !theme.implemented);
    expect(dummyThemes).toHaveLength(9);
  });

  it('resolves all implemented public routes', () => {
    for (const theme of IMPLEMENTED_THEMES) {
      expect(findImplementedThemeByRoute(theme.route.gradeSlug, theme.route.themeSlug)).toBe(theme);
    }
    expect(findImplementedThemeByRoute('grade-2', 'Dummy1')).toBeUndefined();
  });
});
