import { defineTheme, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_1_DEFINITION = defineTheme({
  numeric_theme_id: 2, label: '一次方程式(1)：基本形',
  route: { themeSlug: 'linear-equation-1' },
  search: { title: '一次方程式(1) | AutoDrill', description: '中学1年生向けの基本形の一次方程式ドリルです。' },
  worksheet: { title: '一次方程式(1)：基本形', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
