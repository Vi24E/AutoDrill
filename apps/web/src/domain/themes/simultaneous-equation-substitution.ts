import { defineTheme } from './theme-definition';

export const SIMULTANEOUS_EQUATION_SUBSTITUTION_DEFINITION = defineTheme({
  numeric_theme_id: 71,
  label: '連立方程式（代入法）',
  route: { themeSlug: 'simultaneous-equation-substitution' },
  search: { title: '連立方程式（代入法） | AutoDrill', description: '中学2年生向けに、x=…・y=…の形を利用して代入法で解く連立方程式を練習するドリルです。' },
  worksheet: { title: '連立方程式（代入法）', instruction: '次の連立方程式を代入法で解きなさい。', answerPrefix: null },
});
