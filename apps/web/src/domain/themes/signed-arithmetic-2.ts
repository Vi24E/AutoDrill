import { arithmeticTheme } from './theme-definition';

export const SIGNED_ARITHMETIC_2_DEFINITION = arithmeticTheme({
  numeric_theme_id: 8, label: '正負の数の四則計算（まとめ(1)：整数中心）',
  route: { themeSlug: 'signed-arithmetic-summary-1' },
  search: { title: '正負の数の四則計算（まとめ(1)：整数中心） | AutoDrill', description: '中学1年生向けに、正負の整数を中心として四則計算を総合練習するドリルです。' },
  title: '正負の数の四則計算（まとめ(1)：整数中心）', instruction: '次の式の計算結果を書きなさい。', answerPlacement: 'below',
});
