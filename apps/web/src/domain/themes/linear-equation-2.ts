import { defineTheme, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_2_DEFINITION = defineTheme({
  numeric_theme_id: 3, label: '一次方程式(2)',
  route: { themeSlug: 'linear-equation-2' },
  search: { title: '一次方程式(2) | AutoDrill', description: '中学1年生向けの分数係数・分数解を含む一次方程式ドリルです。' },
  worksheet: { title: '一次方程式(2)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
