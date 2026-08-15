import { FRACTION_SUBTRACTION_CURRICULUM_PATH, FRACTION_SUBTRACTION_GENERATOR_REVISION, FRACTION_SUBTRACTION_LAYOUT, FRACTION_SUBTRACTION_SKILL_ID, FRACTION_SUBTRACTION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, FRACTION_INPUT, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_SUBTRACTION_THEME_ID, generator_revision: FRACTION_SUBTRACTION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.fraction.subtraction', label: '分数の引き算', grade: { slug: 'grade-5', label: '小学5年生' },
  tags: ['fractions', 'subtraction'],
  problemCount: FRACTION_SUBTRACTION_LAYOUT.problem_count, layout: FRACTION_SUBTRACTION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'fraction-subtraction', pathname: '/drills/grade-5/fraction-subtraction' },
  search: { title: '分数の引き算 | AutoDrill', description: '小学5年生向けの正の分数の引き算ドリルです。' },
  compatibility: { skillId: FRACTION_SUBTRACTION_SKILL_ID, curriculumPath: FRACTION_SUBTRACTION_CURRICULUM_PATH },
  inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の引き算', instruction: FRACTION_INSTRUCTION,
});
