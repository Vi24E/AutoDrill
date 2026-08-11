import { MULTIPLICATION_TABLE_CURRICULUM_PATH, MULTIPLICATION_TABLE_GENERATOR_REVISION, MULTIPLICATION_TABLE_LAYOUT, MULTIPLICATION_TABLE_SKILL_ID, MULTIPLICATION_TABLE_THEME_ID } from '../drill-engine';
import { arithmeticTheme, SIMPLE_POSITIVE } from './theme-definition';

export const MULTIPLICATION_TABLE_DEFINITION = arithmeticTheme({
  numeric_theme_id: MULTIPLICATION_TABLE_THEME_ID, generator_revision: MULTIPLICATION_TABLE_GENERATOR_REVISION,
  themeKey: 'jp.grade2.multiplication.table', label: '九九', grade: { slug: 'grade-2', label: '小学2年生' },
  gradeGenre: { genreKey: 'multiplication-and-division', label: '掛け算と割り算' }, recommendedGenre: { genreKey: 'multiplication-and-division', label: '掛け算と割り算' },
  problemCount: MULTIPLICATION_TABLE_LAYOUT.problem_count, layout: MULTIPLICATION_TABLE_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'multiplication-table', pathname: '/drills/grade-2/multiplication-table' },
  search: { title: '九九 | AutoDrill', description: '小学2年生向けの九九ドリルです。' },
  compatibility: { skillId: MULTIPLICATION_TABLE_SKILL_ID, curriculumPath: MULTIPLICATION_TABLE_CURRICULUM_PATH },
  inputInterface: SIMPLE_POSITIVE, title: '九九',
});
