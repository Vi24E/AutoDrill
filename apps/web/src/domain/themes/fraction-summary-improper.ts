import { FRACTION_SUMMARY_IMPROPER_CURRICULUM_PATH, FRACTION_SUMMARY_IMPROPER_GENERATOR_REVISION, FRACTION_SUMMARY_IMPROPER_LAYOUT, FRACTION_SUMMARY_IMPROPER_SKILL_ID, FRACTION_SUMMARY_IMPROPER_THEME_ID } from '../drill-engine';
import { arithmeticTheme, FRACTION_INPUT, IMPROPER_FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUMMARY_IMPROPER_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_SUMMARY_IMPROPER_THEME_ID, generator_revision: FRACTION_SUMMARY_IMPROPER_GENERATOR_REVISION,
  themeKey: 'jp.grade6.fraction.summary_improper', label: '分数総まとめ(仮分数)', grade: { slug: 'grade-6', label: '小学6年生' },
  tags: ['fractions', 'addition', 'subtraction', 'multiplication', 'division'],
  problemCount: FRACTION_SUMMARY_IMPROPER_LAYOUT.problem_count, layout: FRACTION_SUMMARY_IMPROPER_LAYOUT,
  route: { gradeSlug: 'grade-6', themeSlug: 'fraction-summary-improper', pathname: '/drills/grade-6/fraction-summary-improper' },
  search: { title: '分数総まとめ(仮分数) | AutoDrill', description: '小学6年生向けに足し算・引き算・掛け算・割り算を仮分数表記で練習する総まとめドリルです。' },
  compatibility: { skillId: FRACTION_SUMMARY_IMPROPER_SKILL_ID, curriculumPath: FRACTION_SUMMARY_IMPROPER_CURRICULUM_PATH },
  inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数総まとめ(仮分数)', instruction: IMPROPER_FRACTION_INSTRUCTION,
});
