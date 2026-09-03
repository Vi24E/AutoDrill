import { defineTheme } from './theme-definition';

export const LINEAR_EXPRESSION_DEFINITION = defineTheme({
  numeric_theme_id: 75,
  label: '一次式の整理・加減',
  route: { themeSlug: 'linear-expression' },
  search: {
    title: '一次式の整理・加減 | AutoDrill',
    description: '中学1年生向けに、同類項をまとめて一次式を簡単にするドリルです。',
  },
  worksheet: {
    title: '一次式の整理・加減',
    instruction: '次の式を簡単にしなさい。',
    answerPrefix: null,
    answerPlacement: 'below',
  },
});
