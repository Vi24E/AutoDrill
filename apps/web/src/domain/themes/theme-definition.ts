import type { AnswerInputInterface, AnswerInputStructure, CurriculumPathSegment, ProblemPrompt, WorksheetLayout } from '../drill-engine';

export type ThemePromptKind = ProblemPrompt['kind'];
export type GradeSlug = `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`;
export type RouteGroupSlug = GradeSlug | 'bonus';

/**
 * One complete Web projection of a Rust-owned theme. Each theme lives in one file;
 * this object owns only presentation/routing/compatibility metadata, never mathematics.
 */
export type ThemeDefinition = {
  numeric_theme_id: number;
  generator_revision: number;
  themeKey: string;
  label: string;
  grade: { slug: GradeSlug; label: string } | null;
  gradeGenre: { genreKey: string; label: string } | null;
  recommendedGenre: { genreKey: string; label: string } | null;
  problemCount: number;
  layout: WorksheetLayout;
  route: { gradeSlug: RouteGroupSlug; themeSlug: string; pathname: `/drills/${RouteGroupSlug}/${string}` };
  search: { title: string; description: string };
  compatibility: { skillId: string; curriculumPath: readonly CurriculumPathSegment[] };
  promptKind: ThemePromptKind;
  answerSchemaKind: 'integer' | 'rational' | 'decimal' | 'ordered_pair' | 'algebraic';
  inputInterface: AnswerInputInterface;
  worksheet: { title: string; instruction: string; answerPrefix: string | null; answerPlacement?: 'inline' | 'below' };
};

export const ALL_MATH_STRUCTURES = ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'] as const satisfies readonly AnswerInputStructure[];
export const SIMPLE_POSITIVE: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: false };
export const SIMPLE_SIGNED: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: true };
export const SIMPLE_DECIMAL: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: true, allow_negative: false };
export const FRACTION_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction', 'decimal'] };
export const LINEAR_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ALL_MATH_STRUCTURES };
export const QUADRATIC_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction', 'root', 'negative', 'plus_minus', 'tuple', 'arithmetic'] };
export const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';
export const FRACTION_INSTRUCTION = '次の計算をしなさい。答えは約分して最も簡単な分数で答えなさい。';

export function arithmeticTheme(base: Omit<ThemeDefinition, 'promptKind' | 'answerSchemaKind' | 'worksheet'> & {
  answerSchemaKind?: 'integer' | 'rational' | 'decimal' | 'algebraic';
  title: string;
  instruction?: string;
  answerPlacement?: 'inline' | 'below';
}): ThemeDefinition {
  const { title, instruction = '', answerPlacement = 'inline', answerSchemaKind = 'integer', ...rest } = base;
  return { ...rest, promptKind: 'arithmetic', answerSchemaKind, worksheet: { title, instruction, answerPrefix: null, answerPlacement } };
}
