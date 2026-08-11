import type { AnswerInputInterface, AnswerInputStructure, CurriculumPathSegment, ProblemPrompt, WorksheetLayout } from '../drill-engine';

export type ThemePromptKind = ProblemPrompt['kind'];
export type GradeSlug = `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`;

/**
 * One complete Web projection of a Rust-owned theme. Each theme lives in one file;
 * this object owns only presentation/routing/compatibility metadata, never mathematics.
 */
export type ThemeDefinition = {
  numeric_theme_id: number;
  generator_revision: number;
  themeKey: string;
  label: string;
  grade: { slug: GradeSlug; label: string };
  gradeGenre: { genreKey: string; label: string };
  recommendedGenre: { genreKey: string; label: string } | null;
  problemCount: number;
  layout: WorksheetLayout;
  route: { gradeSlug: GradeSlug; themeSlug: string; pathname: `/drills/${GradeSlug}/${string}` };
  search: { title: string; description: string };
  compatibility: { skillId: string; curriculumPath: readonly CurriculumPathSegment[] };
  promptKind: ThemePromptKind;
  answerSchemaKind: 'integer' | 'rational';
  inputInterface: AnswerInputInterface;
  worksheet: { title: string; instruction: string; answerPrefix: string | null };
};

export const ALL_MATH_STRUCTURES = ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'] as const satisfies readonly AnswerInputStructure[];
export const SIMPLE_POSITIVE: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: false };
export const SIMPLE_SIGNED: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: true };
export const FRACTION_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction'] };
export const LINEAR_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ALL_MATH_STRUCTURES };
export const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';
export const FRACTION_INSTRUCTION = '次の計算をしなさい。答えは約分して最も簡単な分数で答えなさい。';

export function arithmeticTheme(base: Omit<ThemeDefinition, 'promptKind' | 'answerSchemaKind' | 'worksheet'> & {
  answerSchemaKind?: 'integer' | 'rational';
  title: string;
  instruction?: string;
}): ThemeDefinition {
  const { title, instruction = '', answerSchemaKind = 'integer', ...rest } = base;
  return { ...rest, promptKind: 'arithmetic', answerSchemaKind, worksheet: { title, instruction, answerPrefix: null } };
}
