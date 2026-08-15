import { FRACTION_INTEGER_MULTIPLICATION_CURRICULUM_PATH, FRACTION_INTEGER_MULTIPLICATION_GENERATOR_REVISION, FRACTION_INTEGER_MULTIPLICATION_LAYOUT, FRACTION_INTEGER_MULTIPLICATION_SKILL_ID, FRACTION_INTEGER_MULTIPLICATION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, FRACTION_INPUT, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_INTEGER_MULTIPLICATION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_INTEGER_MULTIPLICATION_THEME_ID, generator_revision: FRACTION_INTEGER_MULTIPLICATION_GENERATOR_REVISION,
  themeKey: 'jp.grade6.fraction.integer_multiplication', label: '分数と整数の掛け算', grade: { slug: 'grade-6', label: '小学6年生' },
  tags: ['fractions', 'multiplication'],
  problemCount: FRACTION_INTEGER_MULTIPLICATION_LAYOUT.problem_count, layout: FRACTION_INTEGER_MULTIPLICATION_LAYOUT,
  route: { gradeSlug: 'grade-6', themeSlug: 'fraction-integer-multiplication', pathname: '/drills/grade-6/fraction-integer-multiplication' },
  search: { title: '分数と整数の掛け算 | AutoDrill', description: '小学6年生向けの分数と整数の掛け算ドリルです。' },
  compatibility: { skillId: FRACTION_INTEGER_MULTIPLICATION_SKILL_ID, curriculumPath: FRACTION_INTEGER_MULTIPLICATION_CURRICULUM_PATH },
  inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数と整数の掛け算', instruction: FRACTION_INSTRUCTION,
});
