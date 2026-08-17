import { arithmeticTheme, IMPROPER_FRACTION_INSTRUCTION } from './theme-definition';

export const FRACTION_SUMMARY_IMPROPER_DEFINITION = arithmeticTheme({
  numeric_theme_id: 23, label: '分数総まとめ(仮分数)',
  route: { themeSlug: 'fraction-summary-improper' },
  search: { title: '分数総まとめ(仮分数) | AutoDrill', description: '小学6年生向けに足し算・引き算・掛け算・割り算を仮分数表記で練習する総まとめドリルです。' }, title: '分数総まとめ(仮分数)', instruction: IMPROPER_FRACTION_INSTRUCTION,
});
