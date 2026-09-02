import { defineTheme } from './theme-definition';

export const SIMULTANEOUS_EQUATION_ELIMINATION_DEFINITION = defineTheme({
  numeric_theme_id: 19,
  label: '連立方程式（加減法）',
  route: { themeSlug: 'simultaneous-equation-elimination' },
  search: { title: '連立方程式（加減法） | AutoDrill', description: '中学2年生向けに、係数をそろえて加減法で解く連立方程式を練習するドリルです。' },
  worksheet: { title: '連立方程式（加減法）', instruction: '次の連立方程式を加減法で解きなさい。', answerPrefix: null },
});
