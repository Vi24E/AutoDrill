import { defineTheme } from './theme-definition';

export const QUADRATIC_EQUATION_1_DEFINITION = defineTheme({
  numeric_theme_id: 14, label: '二次方程式(1)',
  route: { themeSlug: 'quadratic-equation-1' },
  search: { title: '二次方程式(1) | AutoDrill', description: '中学3年生向けに、ax²+b=0型とa(x+c)²+b=0型を平方根で解く二次方程式ドリルです。' },
  worksheet: { title: '二次方程式(1)', instruction: '次の二次方程式を解きなさい。', answerPrefix: 'x =' },
});
