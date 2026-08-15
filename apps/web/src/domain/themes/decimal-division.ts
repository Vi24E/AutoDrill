import { DECIMAL_DIVISION_CURRICULUM_PATH, DECIMAL_DIVISION_GENERATOR_REVISION, DECIMAL_DIVISION_LAYOUT, DECIMAL_DIVISION_SKILL_ID, DECIMAL_DIVISION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const DECIMAL_DIVISION_DEFINITION = arithmeticTheme({
  numeric_theme_id: DECIMAL_DIVISION_THEME_ID, generator_revision: DECIMAL_DIVISION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.decimal.division', label: '小数の割り算', grade: { slug: 'grade-5', label: '小学5年生' },
  tags: ['decimals', 'division'],
  problemCount: DECIMAL_DIVISION_LAYOUT.problem_count, layout: DECIMAL_DIVISION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'decimal-division', pathname: '/drills/grade-5/decimal-division' },
  search: { title: '小数の割り算 | AutoDrill', description: '小学5年生向けの小数の割り算ドリルです。割り切れる問題を生成します。' },
  compatibility: { skillId: DECIMAL_DIVISION_SKILL_ID, curriculumPath: DECIMAL_DIVISION_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数の割り算', instruction: '次の計算をしなさい。',
});
