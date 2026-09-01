import { arithmeticTheme } from './theme-definition';

export const SIGNED_MULTIPLY_DIVIDE_DEFINITION = arithmeticTheme({
  numeric_theme_id: 67, label: '正負の数の乗法・除法',
  route: { themeSlug: 'signed-multiply-divide' },
  search: { title: '正負の数の乗法・除法 | AutoDrill', description: '中学1年生向けに、正負の整数の乗法と除法を練習する計算ドリルです。' },
  title: '正負の数の乗法・除法', instruction: '次の式の計算結果を書きなさい。', answerPlacement: 'below',
});
