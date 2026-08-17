import { defineTheme } from './theme-definition';

export const LIAR_PUZZLE_DEFINITION = defineTheme({
  numeric_theme_id: 20,
  label: 'うそつきだれだ',
  route: { themeSlug: 'liar-puzzle' },
  search: { title: 'うそつきだれだ | AutoDrill', description: '正直者とうそつきの発言から、うそつきを全員見つける論理クイズです。' },
  worksheet: { title: 'うそつきだれだ', instruction: '正直な人は本当のことを、うそつきはうそのことを言います。うそつきを全員選びなさい。', answerPrefix: null },
});
