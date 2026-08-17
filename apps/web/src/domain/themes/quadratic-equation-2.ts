import { defineTheme } from './theme-definition';

export const QUADRATIC_EQUATION_2_DEFINITION = defineTheme({
  numeric_theme_id: 15, label: '二次方程式(2)',
  route: { themeSlug: 'quadratic-equation-2' },
  search: { title: '二次方程式(2) | AutoDrill', description: '中学3年生向けの因数分解で解く二次方程式ドリルです。' },
  worksheet: { title: '二次方程式(2)', instruction: '次の二次方程式を解きなさい。', answerPrefix: 'x =' },
});
