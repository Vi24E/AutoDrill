import { defineTheme } from './theme-definition';

export const QUADRATIC_EQUATION_3_DEFINITION = defineTheme({
  numeric_theme_id: 16, label: '二次方程式(3)',
  route: { themeSlug: 'quadratic-equation-3' },
  search: { title: '二次方程式(3) | AutoDrill', description: '中学3年生向けに、整数係数の二次方程式を解の公式で解くドリルです。' },
  worksheet: { title: '二次方程式(3)', instruction: '次の二次方程式を解きなさい。必要なら解の公式を使いなさい。', answerPrefix: 'x =' },
});
