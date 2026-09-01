import { arithmeticTheme } from './theme-definition';

export const DIVISION_WITH_REMAINDER_DEFINITION = arithmeticTheme({
  numeric_theme_id: 52, label: 'あまりのある割り算',
  route: { themeSlug: 'division-with-remainder' },
  search: { title: 'あまりのある割り算 | AutoDrill', description: '小学3年生向けの、商が1桁であまりのある割り算ドリルです。' }, title: 'あまりのある割り算', instruction: '商とあまりを答えなさい。',
});
