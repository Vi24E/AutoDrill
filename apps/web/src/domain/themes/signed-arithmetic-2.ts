import { SIGNED_ARITHMETIC_2_CURRICULUM_PATH, SIGNED_ARITHMETIC_2_GENERATOR_REVISION, SIGNED_ARITHMETIC_2_LAYOUT, SIGNED_ARITHMETIC_2_SKILL_ID, SIGNED_ARITHMETIC_2_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIGNED_RATIONAL_INPUT } from './theme-definition';

export const SIGNED_ARITHMETIC_2_DEFINITION = arithmeticTheme({
  numeric_theme_id: SIGNED_ARITHMETIC_2_THEME_ID, generator_revision: SIGNED_ARITHMETIC_2_GENERATOR_REVISION,
  themeKey: 'jp.grade7.signed.arithmetic.2', label: '負の数の計算(2)', grade: { slug: 'grade-7', label: '中学1年生' },
  tags: ['negative_numbers'],
  problemCount: SIGNED_ARITHMETIC_2_LAYOUT.problem_count, layout: SIGNED_ARITHMETIC_2_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-2', pathname: '/drills/grade-7/signed-arithmetic-2' },
  search: { title: '負の数の計算(2) | AutoDrill', description: '中学1年生向けの正負の数の四則計算ドリルです。整数だけでなく小さい分数の答えも扱います。' },
  compatibility: { skillId: SIGNED_ARITHMETIC_2_SKILL_ID, curriculumPath: SIGNED_ARITHMETIC_2_CURRICULUM_PATH },
  inputInterface: SIGNED_RATIONAL_INPUT, answerSchemaKind: 'rational', title: '負の数の計算(2)', instruction: '次の式を計算しなさい。', answerPlacement: 'below',
});
