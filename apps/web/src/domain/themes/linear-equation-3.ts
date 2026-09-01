import { defineTheme, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_3_DEFINITION = defineTheme({
  numeric_theme_id: 70, label: '一次方程式(3)：括弧・分数・小数係数',
  route: { themeSlug: 'linear-equation-3' },
  search: { title: '一次方程式(3) | AutoDrill', description: '中学1年生向けの括弧・分数係数・小数係数を含む一次方程式ドリルです。' },
  worksheet: { title: '一次方程式(3)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
