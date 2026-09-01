import { defineTheme, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_2_DEFINITION = defineTheme({
  numeric_theme_id: 3, label: '一次方程式(2)：括弧・整数係数中心',
  route: { themeSlug: 'linear-equation-2' },
  search: { title: '一次方程式(2) | AutoDrill', description: '中学1年生向けの括弧を展開して解く、整数係数中心の一次方程式ドリルです。' },
  worksheet: { title: '一次方程式(2)：括弧・整数係数中心', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
