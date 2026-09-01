import { defineTheme } from './theme-definition';

export const ONE_DIGIT_ADDITION_DEFINITION = defineTheme({
  numeric_theme_id: 1,
  label: '一桁の足し算（まとめ）',
  route: { themeSlug: 'one-digit-addition' },
  search: { title: '一桁の足し算 | AutoDrill', description: '小学1年生向けの一桁の足し算ドリルです。' },
  worksheet: { title: '1けたのたしざん(1)', instruction: '', answerPrefix: null },
});
