import { describe, expect, it } from 'vitest';

import {
  CURRICULUM_TREE,
  DEFAULT_WEB_DRILL_SETTINGS,
  IMPLEMENTED_THEMES,
  LINEAR_EQUATION_SIMPLE_THEME,
  LINEAR_EQUATION_1_THEME,
  LINEAR_EQUATION_2_THEME,
  LINEAR_EQUATION_3_THEME,
  ONE_DIGIT_ADDITION_THEME,
  RECOMMENDED_GENRES,
  findImplementedThemeByRoute,
  type ImplementedCurriculumTheme,
} from '@/domain/curriculum';
import { taxonomyTags } from '@/domain/theme-registry';
import { DRILL_SCHEMA_VERSION } from '@/domain/drill-engine';

describe('Web curriculum registry', () => {
  it('registers all implemented arithmetic and equation themes from one data model', () => {
    expect(IMPLEMENTED_THEMES.map((theme) => theme.numeric_theme_id).sort((a, b) => a - b)).toEqual(Array.from({ length: 75 }, (_, index) => index + 1));
    expect(ONE_DIGIT_ADDITION_THEME).toMatchObject({
      numeric_theme_id: 1,
      generator_revision: 5,
      tags: ['addition'],
      recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
      problemCount: 20,
      layout: { problem_count: 20, columns: 2, rows: 10 },
    });
    const symbolic = IMPLEMENTED_THEMES.find((theme) => theme.numeric_theme_id === 75)!;
    expect(symbolic).toMatchObject({
      grade: { slug: 'grade-7', label: '中学1年生' },
      curriculumUnit: { unitKey: 'grade7-expressions', label: '文字を用いた式' },
      recommendedGenre: { genreKey: 'expressions', label: '文字式' },
      promptKind: 'linear_expression',
      answerSchemaKind: 'linear_expression',
      problemCount: 16,
      layout: { problem_count: 16, columns: 2, rows: 8 },
      worksheet: { title: '一次式の整理・加減', instruction: '次の式を簡単にしなさい。', answerPrefix: null },
    });
    expect(symbolic.inputInterface).toEqual({
      type: 'structured_math',
      allowed_structures: ['negative', 'arithmetic', 'variable'],
    });
    expect(symbolic.editorInputInterface).toEqual(symbolic.inputInterface);

    for (const theme of [LINEAR_EQUATION_SIMPLE_THEME, LINEAR_EQUATION_1_THEME, LINEAR_EQUATION_2_THEME, LINEAR_EQUATION_3_THEME]) {
      expect(theme).toMatchObject({
        grade: { slug: 'grade-7', label: '中学1年生' },
        curriculumUnit: expect.objectContaining({ label: expect.any(String) }),
        recommendedGenre: { genreKey: 'equation', label: '方程式' },
        problemCount: 16,
        layout: { problem_count: 16, columns: 2, rows: 8 },
        promptKind: 'linear_equation',
        worksheet: { answerPrefix: 'x =' },
      });
      expect(theme.inputInterface).toEqual({
        type: 'structured_math',
        allowed_structures: ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'],
      });
      expect(theme.editorInputInterface).toEqual({
        type: 'structured_math',
        allowed_structures: ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple', 'arithmetic'],
      });
    }
  });

  it('exposes the implemented Recommended sections and defaults to addition', () => {
    expect(DEFAULT_WEB_DRILL_SETTINGS).toEqual({
      schema_version: DRILL_SCHEMA_VERSION,
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
      { genreKey: 'expressions', label: '文字式' },
      { genreKey: 'equation', label: '方程式' },
      { genreKey: 'bonus', label: 'おまけ' },
    ]);
    expect(RECOMMENDED_GENRES.flatMap((genre) => genre.themes).map((theme) => theme.numeric_theme_id).sort((a, b) => a - b)).toEqual(IMPLEMENTED_THEMES.map((theme) => theme.numeric_theme_id).sort((a, b) => a - b));
    const equationRecommended = RECOMMENDED_GENRES.find((genre) => genre.genreKey === 'equation')!;
    expect(equationRecommended.themes.map((theme) => theme.label)).toContain('連立方程式（まとめ(1)）');
    expect(equationRecommended.themes.map((theme) => theme.label)).toContain('連立方程式（まとめ(2)）');
    const signedRecommended = RECOMMENDED_GENRES.find((genre) => genre.genreKey === 'negative-numbers')!;
    expect(signedRecommended.themes.map((theme) => theme.label)).toEqual([
      '正負の数の加法・減法',
      '正負の数の乗法・除法',
      '正負の数の四則計算（まとめ(1)：整数中心）',
      '正負の数の四則計算（まとめ(2)：小数・分数を含む）',
    ]);
  });

  it('keeps grade curriculum units independent from Recommended taxonomy tags', () => {
    const columnThemes = IMPLEMENTED_THEMES.filter((theme) => theme.presentation.column_arithmetic);
    expect(columnThemes).toHaveLength(22);
    for (const theme of columnThemes) {
      expect(theme.presentation.print_recommended).toBe(true);
      expect(theme.presentation.worksheet_grid).toBe(true);
      const isDivision = theme.tags.includes('division');
      expect(theme.problemCount).toBe(isDivision ? 12 : 16);
      expect(theme.layout).toEqual(isDivision
        ? { problem_count: 12, columns: 4, rows: 3 }
        : { problem_count: 16, columns: 4, rows: 4 });
      expect(theme.curriculumUnit).not.toBeNull();
      expect(theme.recommendedGenre).not.toBeNull();
      const tags = taxonomyTags(theme);
      expect(tags.some((tag) => tag.startsWith('grade_') || tag.startsWith('junior_high_'))).toBe(true);
    }
    const grade2 = columnThemes.find((theme) => theme.numeric_theme_id === 25)!;
    expect(taxonomyTags(grade2)).toContain('grade_2');
    expect(grade2.curriculumUnit).toEqual({ unitKey: 'grade2-column-add-subtract', label: '加法，減法' });
    const decimal = columnThemes.find((theme) => theme.numeric_theme_id === 33)!;
    expect(decimal.recommendedGenre).toEqual({ genreKey: 'decimals', label: '小数' });

    const miniSudoku = IMPLEMENTED_THEMES.find((theme) => theme.numeric_theme_id === 38)!;
    expect(miniSudoku.presentation).toMatchObject({
      worksheet_grid: true,
      column_arithmetic: false,
      print_recommended: false,
    });
    expect(miniSudoku.layout).toEqual({ problem_count: 4, columns: 2, rows: 2 });
    expect(miniSudoku.inputInterface).toEqual({ type: 'digit_grid', min_digit: 1, max_digit: 4, cell_count: 16 });
  });

  it('never exposes negative input capability for elementary-school themes', () => {
    const elementary = IMPLEMENTED_THEMES.filter((theme) => theme.grade && theme.grade.number <= 6);
    expect(elementary.length).toBeGreaterThan(0);
    for (const theme of elementary) {
      if (theme.inputInterface.type === 'simple_numeric') {
        expect(theme.inputInterface.allow_negative).toBe(false);
      } else if (theme.inputInterface.type === 'structured_math') {
        expect(theme.inputInterface.allowed_structures).not.toContain('negative');
        expect(theme.inputInterface.allowed_structures).not.toContain('plus_minus');
      } else {
        expect(theme.inputInterface.min_digit).toBeGreaterThanOrEqual(0);
      }
    }
  });

  it('maps grade-1 through grade-9 and groups multiplication-table siblings as one curriculum unit', () => {
    expect(CURRICULUM_TREE.map((grade) => grade.slug)).toEqual([
      'grade-1', 'grade-2', 'grade-3', 'grade-4', 'grade-5',
      'grade-6', 'grade-7', 'grade-8', 'grade-9',
    ]);
    expect(CURRICULUM_TREE.map((grade) => grade.label)).toEqual([
      '小学1年生', '小学2年生', '小学3年生', '小学4年生', '小学5年生',
      '小学6年生', '中学1年生', '中学2年生', '中学3年生',
    ]);
    for (const grade of CURRICULUM_TREE) {
      expect(grade.units.length, grade.label).toBeGreaterThan(0);
      expect(grade.units.flatMap((unit) => unit.themes).every((theme) => theme.implemented), grade.label).toBe(true);
    }

    const grade7 = CURRICULUM_TREE[6]!;
    const signedNumbers = grade7.units.find((unit) => unit.unitKey === 'signed-numbers')!;
    expect(signedNumbers.label).toBe('正負の数');
    expect(signedNumbers.themes.map((theme) => theme.label)).toEqual([
      '正負の数の加法・減法',
      '正負の数の乗法・除法',
      '正負の数の四則計算（まとめ(1)：整数中心）',
      '正負の数の四則計算（まとめ(2)：小数・分数を含む）',
    ]);
    const signedThemes = signedNumbers.themes
      .filter((theme): theme is ImplementedCurriculumTheme => theme.implemented);
    expect(signedThemes.map((theme) => theme.numeric_theme_id)).toEqual([7, 67, 8, 68]);
    expect(signedThemes.slice(0, 3).every((theme) => theme.answerSchemaKind === 'integer' || theme.answerSchemaKind === 'rational')).toBe(true);
    expect(signedThemes[3]!.inputInterface).toEqual({
      type: 'structured_math',
      allowed_structures: ['fraction', 'negative'],
    });

    const linearEquation = grade7.units.find((unit) => unit.unitKey === 'linear-equation')!;
    expect(linearEquation.label).toBe('一次方程式');
    expect(linearEquation.themes).toEqual([LINEAR_EQUATION_SIMPLE_THEME, LINEAR_EQUATION_1_THEME, LINEAR_EQUATION_2_THEME, LINEAR_EQUATION_3_THEME]);
    expect(linearEquation.themes.map((theme) => theme.label)).toEqual([
      '簡単な一次方程式',
      '一次方程式(1)：基本形',
      '一次方程式(2)：括弧・整数係数中心',
      '一次方程式(3)：括弧・分数・小数係数',
    ]);

    const grade3 = CURRICULUM_TREE[2]!;
    const grade3Themes = grade3.units
      .flatMap((unit) => unit.themes)
      .filter((theme): theme is ImplementedCurriculumTheme => theme.implemented);
    expect(grade3Themes.some((theme) => (
      theme.presentation.column_arithmetic && theme.tags.includes('division')
    ))).toBe(false);

    const grade4 = CURRICULUM_TREE[3]!;
    const sameDenominatorFractions = grade4.units.find((unit) => unit.unitKey === 'grade4-fraction-add-subtract')!;
    expect(sameDenominatorFractions.label).toBe('同分母の分数の加法，減法');
    expect(sameDenominatorFractions.themes.map((theme) => theme.label)).toEqual([
      '同分母の分数の足し算', '同分母の分数の引き算',
    ]);

    const grade5 = CURRICULUM_TREE[4]!;
    const fractionAddSubtract = grade5.units.find((unit) => unit.unitKey === 'grade5-fraction-add-subtract')!;
    expect(fractionAddSubtract.label).toBe('分数の加法，減法');
    expect(fractionAddSubtract.themes.map((theme) => theme.label)).toEqual([
      '異分母の分数の足し算', '異分母の分数の引き算',
      '分数の足し算（まとめ）', '分数の引き算（まとめ）',
    ]);

    const grade5Decimal = grade5.units.find((unit) => unit.unitKey === 'grade5-decimal')!;
    expect(grade5Decimal.themes.map((theme) => theme.label)).toEqual([
      '小数の掛け算の筆算',
      '小数の割り算の筆算',
      '余りを答える小数の割り算の筆算',
      '商を四捨五入する小数の割り算の筆算',
    ]);
    const remainderTheme = grade5Decimal.themes.find((theme): theme is ImplementedCurriculumTheme => (
      theme.implemented && theme.label === '余りを答える小数の割り算の筆算'
    ))!;
    expect(remainderTheme.answerSchemaKind).toBe('ordered_pair');
    expect(remainderTheme.inputInterface).toEqual({
      type: 'structured_math',
      allowed_structures: ['decimal', 'tuple'],
    });
    const roundedTheme = grade5Decimal.themes.find((theme): theme is ImplementedCurriculumTheme => (
      theme.implemented && theme.label === '商を四捨五入する小数の割り算の筆算'
    ))!;
    expect(roundedTheme.answerSchemaKind).toBe('decimal');
    expect(roundedTheme.inputInterface).toEqual({
      type: 'simple_numeric',
      allow_decimal: true,
      allow_negative: false,
    });

    const integerDivision = grade4.units.find((unit) => unit.unitKey === 'grade4-integer-division')!;
    const integerDivisionThemes = integerDivision.themes
      .filter((theme): theme is ImplementedCurriculumTheme => theme.implemented);
    expect(integerDivision.label).toBe('整数の除法');
    expect(integerDivisionThemes.map((theme) => theme.label)).toEqual([
      '2桁÷1桁の筆算', '3桁÷1桁の筆算', '二桁で割る割り算の筆算',
    ]);
    expect(integerDivisionThemes.slice(0, 2).map((theme) => theme.grade?.number)).toEqual([4, 4]);

    const grade4Decimal = grade4.units.find((unit) => unit.unitKey === 'grade4-decimal')!;
    expect(grade4Decimal.themes.map((theme) => theme.label)).toEqual([
      '小数の足し算と引き算（まとめ）',
      '小数の足し算の筆算',
      '小数の引き算の筆算',
      '小数と整数の掛け算の筆算',
      '小数と整数の割り算の筆算',
    ]);

    const grade2 = CURRICULUM_TREE[1]!;
    const grade2ColumnAddSubtract = grade2.units.find((unit) => unit.unitKey === 'grade2-column-add-subtract')!;
    expect(grade2ColumnAddSubtract.themes.map((theme) => theme.label)).toEqual([
      '二桁の足し算の筆算（まとめ）',
      '二桁の引き算の筆算（まとめ）',
      '足し算・繰り上がりなし',
      '足し算・繰り上がりあり',
      '引き算・繰り下がりなし',
      '引き算・繰り下がりあり',
    ]);
    const multiplicationTable = grade2.units.find((unit) => unit.unitKey === 'multiplication-table')!;
    expect(multiplicationTable.label).toBe('九九');
    expect(multiplicationTable.themes.map((theme) => theme.label)).toEqual([
      '全段混合', '1の段', '2の段', '3の段', '4の段', '5の段',
      '6の段', '7の段', '8の段', '9の段',
    ]);
  });

  it('resolves all implemented public routes', () => {
    for (const theme of IMPLEMENTED_THEMES) {
      expect(findImplementedThemeByRoute(theme.route.gradeSlug, theme.route.themeSlug)).toBe(theme);
    }
  });
});