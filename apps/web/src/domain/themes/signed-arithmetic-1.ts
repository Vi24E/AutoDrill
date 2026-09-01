import { arithmeticTheme } from './theme-definition';

export const SIGNED_ARITHMETIC_1_DEFINITION = arithmeticTheme({
  numeric_theme_id: 7, label: '正負の数の加法・減法',
  route: { themeSlug: 'signed-add-subtract' },
  search: { title: '正負の数の加法・減法 | AutoDrill', description: '中学1年生向けに、正負の整数の加法と減法を練習する計算ドリルです。' },
  title: '正負の数の加法・減法', instruction: '次の式の計算結果を書きなさい。', answerPlacement: 'below',
});
