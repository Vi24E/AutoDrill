import { DECIMAL_ADD_SUBTRACT_CURRICULUM_PATH, DECIMAL_ADD_SUBTRACT_GENERATOR_REVISION, DECIMAL_ADD_SUBTRACT_LAYOUT, DECIMAL_ADD_SUBTRACT_SKILL_ID, DECIMAL_ADD_SUBTRACT_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_DECIMAL } from './theme-definition';

export const DECIMAL_ADD_SUBTRACT_DEFINITION = arithmeticTheme({
  numeric_theme_id: DECIMAL_ADD_SUBTRACT_THEME_ID, generator_revision: DECIMAL_ADD_SUBTRACT_GENERATOR_REVISION,
  themeKey: 'jp.grade4.decimal.add_subtract', label: '小数の足し算と引き算', grade: { slug: 'grade-4', label: '小学4年生' },
  gradeGenre: { genreKey: 'decimals', label: '小数' }, recommendedGenre: { genreKey: 'decimals', label: '小数' },
  problemCount: DECIMAL_ADD_SUBTRACT_LAYOUT.problem_count, layout: DECIMAL_ADD_SUBTRACT_LAYOUT,
  route: { gradeSlug: 'grade-4', themeSlug: 'decimal-add-subtract', pathname: '/drills/grade-4/decimal-add-subtract' },
  search: { title: '小数の足し算と引き算 | AutoDrill', description: '小学4年生向けの小数の足し算・引き算ドリルです。' },
  compatibility: { skillId: DECIMAL_ADD_SUBTRACT_SKILL_ID, curriculumPath: DECIMAL_ADD_SUBTRACT_CURRICULUM_PATH },
  inputInterface: SIMPLE_DECIMAL, answerSchemaKind: 'decimal', title: '小数の足し算と引き算', instruction: '次の計算をしなさい。',
});
