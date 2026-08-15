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
import { ALL_MATH_STRUCTURES, taxonomyTags } from '@/domain/theme-registry';

describe('Web curriculum registry', () => {
  it('registers all implemented arithmetic and equation themes from one data model', () => {
    expect(IMPLEMENTED_THEMES.map((theme) => theme.numeric_theme_id)).toEqual([1, 4, 5, 25, 26, 27, 28, 6, 13, 29, 30, 31, 32, 17, 33, 34, 35, 9, 18, 24, 36, 37, 11, 10, 12, 21, 22, 23, 7, 8, 2, 3, 19, 14, 15, 16, 20]);
    expect(ONE_DIGIT_ADDITION_THEME).toMatchObject({
      numeric_theme_id: 1,
      generator_revision: 5,
      tags: ['addition'],
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

  it('exposes the implemented Recommended sections and defaults to addition', () => {
    expect(DEFAULT_WEB_DRILL_SETTINGS).toEqual({
      schema_version: 4,
      numeric_theme_id: ONE_DIGIT_ADDITION_THEME.numeric_theme_id,
      themeKey: ONE_DIGIT_ADDITION_THEME.themeKey,
      difficulty: 2,
      seed: '',
    });
    expect(RECOMMENDED_GENRES.map(({ genreKey, label }) => ({ genreKey, label }))).toEqual([
      { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
      { genreKey: 'multiplication-and-division', label: '掛け算と割り算' },
      { genreKey: 'decimals', label: '小数' },
      { genreKey: 'fractions', label: '分数' },
      { genreKey: 'negative-numbers', label: '負の数' },
      { genreKey: 'equation', label: '方程式' },
      { genreKey: 'bonus', label: 'おまけ' },
    ]);
    expect(RECOMMENDED_GENRES.flatMap((genre) => genre.themes).map((theme) => theme.numeric_theme_id).sort((a, b) => a - b)).toEqual(IMPLEMENTED_THEMES.map((theme) => theme.numeric_theme_id).sort((a, b) => a - b));
  });

  it('derives grade and UI classification from typed taxonomy tags', () => {
    const columnThemes = IMPLEMENTED_THEMES.filter((theme) => theme.tags.includes('column_arithmetic'));
    expect(columnThemes).toHaveLength(13);
    for (const theme of columnThemes) {
      expect(theme.tags).toContain('print_recommended');
      const isDivision = theme.tags.includes('division');
      expect(theme.problemCount).toBe(isDivision ? 12 : 16);
      expect(theme.layout).toEqual(isDivision
        ? { problem_count: 12, columns: 4, rows: 3 }
        : { problem_count: 16, columns: 4, rows: 4 });
      expect(theme.gradeGenre).not.toBeNull();
      expect(theme.recommendedGenre).not.toBeNull();
      const tags = taxonomyTags(theme);
      expect(tags.some((tag) => tag.startsWith('grade_') || tag.startsWith('junior_high_'))).toBe(true);
    }
    const grade2 = columnThemes.find((theme) => theme.numeric_theme_id === 25)!;
    expect(taxonomyTags(grade2)).toContain('grade_2');
    expect(grade2.gradeGenre).toEqual({ genreKey: 'addition-and-subtraction', label: '足し算と引き算' });
    const decimal = columnThemes.find((theme) => theme.numeric_theme_id === 33)!;
    expect(decimal.recommendedGenre).toEqual({ genreKey: 'decimals', label: '小数' });
  });

  it('never exposes negative input capability for elementary-school themes', () => {
    const elementary = IMPLEMENTED_THEMES.filter((theme) => theme.grade && Number(theme.grade.slug.slice(6)) <= 6);
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

  it('maps grade-1 through grade-9 with at least one implemented theme in every grade', () => {
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

    for (const grade of CURRICULUM_TREE) {
      expect(grade.genres.length, grade.label).toBeGreaterThan(0);
      expect(grade.genres.flatMap((genre) => genre.themes).every((theme) => theme.implemented), grade.label).toBe(true);
    }
    const grade8 = CURRICULUM_TREE[7]!;
    const simultaneous = grade8.genres.find((genre) => genre.genreKey === 'simultaneous-equation')!;
    expect(simultaneous.label).toBe('連立方程式');
    expect(simultaneous.themes.map((theme) => theme.label)).toEqual(['連立方程式(1)']);
  });

  it('resolves all implemented public routes', () => {
    for (const theme of IMPLEMENTED_THEMES) {
      expect(findImplementedThemeByRoute(theme.route.gradeSlug, theme.route.themeSlug)).toBe(theme);
    }
  });
});