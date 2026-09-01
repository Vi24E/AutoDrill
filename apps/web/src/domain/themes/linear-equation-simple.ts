import { defineTheme, LINEAR_INSTRUCTION } from './theme-definition';

export const LINEAR_EQUATION_SIMPLE_DEFINITION = defineTheme({
  numeric_theme_id: 69, label: '簡単な一次方程式',
  route: { themeSlug: 'linear-equation-simple' },
  search: { title: '簡単な一次方程式 | AutoDrill', description: '中学1年生向けの x+a=b、ax=b の導入一次方程式ドリルです。' },
  worksheet: { title: '簡単な一次方程式', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
});
