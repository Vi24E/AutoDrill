import { defineTheme } from './theme-definition';

export const MINI_SUDOKU_DEFINITION = defineTheme({
  numeric_theme_id: 38,
  label: 'すうじはひとりぼっち',
  route: { themeSlug: 'mini-sudoku' },
  search: {
    title: 'すうじはひとりぼっち | AutoDrill',
    description: '4×4の小さな数独です。たて・よこ・2×2のブロックに1〜4を1回ずつ入れます。',
  },
  worksheet: {
    title: 'すうじはひとりぼっち',
    instruction: '1〜4の数字を、たて・よこ・太い線で囲まれた2×2の中に1回ずつ入れなさい。',
    answerPrefix: null,
  },
});
