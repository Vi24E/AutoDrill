import { ONE_DIGIT_SUBTRACTION_CURRICULUM_PATH, ONE_DIGIT_SUBTRACTION_GENERATOR_REVISION, ONE_DIGIT_SUBTRACTION_LAYOUT, ONE_DIGIT_SUBTRACTION_SKILL_ID, ONE_DIGIT_SUBTRACTION_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const ONE_DIGIT_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: ONE_DIGIT_SUBTRACTION_THEME_ID, generator_revision: ONE_DIGIT_SUBTRACTION_GENERATOR_REVISION,
  themeKey: 'jp.grade1.subtraction.one_digit', label: '一桁の引き算', grade: { slug: 'grade-1', label: '小学1年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' }, recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: ONE_DIGIT_SUBTRACTION_LAYOUT.problem_count, layout: ONE_DIGIT_SUBTRACTION_LAYOUT,
  route: { gradeSlug: 'grade-1', themeSlug: 'one-digit-subtraction', pathname: '/drills/grade-1/one-digit-subtraction' },
  search: { title: '一桁の引き算 | AutoDrill', description: '小学1年生向けの一桁の引き算ドリルです。' },
  compatibility: { skillId: ONE_DIGIT_SUBTRACTION_SKILL_ID, curriculumPath: ONE_DIGIT_SUBTRACTION_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, title: '1けたのひきざん',
});
