import { FRACTION_ADDITION_CURRICULUM_PATH, FRACTION_ADDITION_GENERATOR_REVISION, FRACTION_ADDITION_LAYOUT, FRACTION_ADDITION_SKILL_ID, FRACTION_ADDITION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, FRACTION_INPUT, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_ADDITION_THEME_ID, generator_revision: FRACTION_ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.fraction.addition', label: '分数の足し算', grade: { slug: 'grade-5', label: '小学5年生' },
  tags: ['fractions', 'addition'],
  problemCount: FRACTION_ADDITION_LAYOUT.problem_count, layout: FRACTION_ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'fraction-addition', pathname: '/drills/grade-5/fraction-addition' },
  search: { title: '分数の足し算 | AutoDrill', description: '小学5年生向けの分数の足し算ドリルです。' },
  compatibility: { skillId: FRACTION_ADDITION_SKILL_ID, curriculumPath: FRACTION_ADDITION_CURRICULUM_PATH },
  inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の足し算', instruction: FRACTION_INSTRUCTION,
});
