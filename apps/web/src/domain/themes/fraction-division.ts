import { FRACTION_DIVISION_CURRICULUM_PATH, FRACTION_DIVISION_GENERATOR_REVISION, FRACTION_DIVISION_LAYOUT, FRACTION_DIVISION_SKILL_ID, FRACTION_DIVISION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, FRACTION_INPUT, FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_DIVISION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_DIVISION_THEME_ID, generator_revision: FRACTION_DIVISION_GENERATOR_REVISION,
  themeKey: 'jp.grade6.fraction.division', label: '分数の割り算', grade: { slug: 'grade-6', label: '小学6年生' },
  gradeGenre: { genreKey: 'fractions', label: '分数' }, recommendedGenre: { genreKey: 'fractions', label: '分数' },
  problemCount: FRACTION_DIVISION_LAYOUT.problem_count, layout: FRACTION_DIVISION_LAYOUT,
  route: { gradeSlug: 'grade-6', themeSlug: 'fraction-division', pathname: '/drills/grade-6/fraction-division' },
  search: { title: '分数の割り算 | AutoDrill', description: '小学6年生向けの分数の割り算ドリルです。' },
  compatibility: { skillId: FRACTION_DIVISION_SKILL_ID, curriculumPath: FRACTION_DIVISION_CURRICULUM_PATH },
  inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の割り算', instruction: FRACTION_INSTRUCTION,
});
