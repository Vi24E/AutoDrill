import { TWO_DIGIT_ADDITION_CURRICULUM_PATH, TWO_DIGIT_ADDITION_GENERATOR_REVISION, TWO_DIGIT_ADDITION_LAYOUT, TWO_DIGIT_ADDITION_SKILL_ID, TWO_DIGIT_ADDITION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const TWO_DIGIT_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: TWO_DIGIT_ADDITION_THEME_ID, generator_revision: TWO_DIGIT_ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade2.addition.two_digit', label: '二桁の足し算', grade: { slug: 'grade-2', label: '小学2年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' }, recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: TWO_DIGIT_ADDITION_LAYOUT.problem_count, layout: TWO_DIGIT_ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'two-digit-addition', pathname: '/drills/grade-2/two-digit-addition' },
  search: { title: '二桁の足し算 | AutoDrill', description: '小学2年生向けの二桁の足し算ドリルです。' },
  compatibility: { skillId: TWO_DIGIT_ADDITION_SKILL_ID, curriculumPath: TWO_DIGIT_ADDITION_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, title: '2けたのたしざん',
});
