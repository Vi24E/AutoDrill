import type { AnswerInputInterface, AnswerInputStructure, CurriculumPathSegment, ProblemPrompt, WorksheetLayout } from '../drill-engine';

export type ThemePromptKind = ProblemPrompt['kind'];
export type GradeSlug = `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`;
export type RouteGroupSlug = GradeSlug | 'bonus';

export const THEME_TAG_VALUES = [
  'addition',
  'subtraction',
  'multiplication',
  'division',
  'fractions',
  'decimals',
  'negative_numbers',
  'equations',
  'linear_equation',
  'simultaneous_equation',
  'quadratic_equation',
  'bonus',
  'column_arithmetic',
  'print_recommended',
] as const;
export type ThemeTag = typeof THEME_TAG_VALUES[number];
export type DerivedGradeTag =
  | 'grade_1' | 'grade_2' | 'grade_3' | 'grade_4' | 'grade_5' | 'grade_6'
  | 'junior_high_1' | 'junior_high_2' | 'junior_high_3';
export type ThemeTaxonomyTag = ThemeTag | DerivedGradeTag;
export type ThemeGenreMetadata = { genreKey: string; label: string };

/**
 * Stored theme metadata. Grade is canonical curriculum metadata; grade tags and
 * UI genres are derived from it/tags so the same classification is not entered
 * again in each theme file.
 */
export type ThemeDefinition = {
  numeric_theme_id: number;
  generator_revision: number;
  themeKey: string;
  label: string;
  grade: { slug: GradeSlug; label: string } | null;
  tags: readonly ThemeTag[];
  /** Derived by defineTheme; retained as a stable read-only projection for callers. */
  gradeGenre: ThemeGenreMetadata | null;
  /** Derived by defineTheme; retained as a stable read-only projection for callers. */
  recommendedGenre: ThemeGenreMetadata | null;
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

export type ThemeDefinitionInput = Omit<ThemeDefinition, 'gradeGenre' | 'recommendedGenre'>;

function hasAny(tags: readonly ThemeTag[], candidates: readonly ThemeTag[]): boolean {
  return candidates.some((tag) => tags.includes(tag));
}

export function gradeGenreFromTags(tags: readonly ThemeTag[]): ThemeGenreMetadata | null {
  if (tags.includes('fractions')) return { genreKey: 'fractions', label: '分数' };
  if (tags.includes('decimals')) return { genreKey: 'decimals', label: '小数' };
  if (tags.includes('negative_numbers')) return { genreKey: 'signed-numbers', label: '正の数・負の数' };
  if (tags.includes('linear_equation')) return { genreKey: 'linear-equation', label: '一次方程式' };
  if (tags.includes('simultaneous_equation')) return { genreKey: 'simultaneous-equation', label: '連立方程式' };
  if (tags.includes('quadratic_equation')) return { genreKey: 'quadratic-equation', label: '二次方程式' };
  if (hasAny(tags, ['addition', 'subtraction'])) return { genreKey: 'addition-and-subtraction', label: '足し算と引き算' };
  if (hasAny(tags, ['multiplication', 'division'])) return { genreKey: 'multiplication-and-division', label: '掛け算と割り算' };
  return null;
}

export function recommendedGenreFromTags(tags: readonly ThemeTag[]): ThemeGenreMetadata | null {
  if (tags.includes('bonus')) return { genreKey: 'bonus', label: 'おまけ' };
  if (tags.includes('equations')) return { genreKey: 'equation', label: '方程式' };
  if (tags.includes('negative_numbers')) return { genreKey: 'negative-numbers', label: '負の数' };
  if (tags.includes('fractions')) return { genreKey: 'fractions', label: '分数' };
  if (tags.includes('decimals')) return { genreKey: 'decimals', label: '小数' };
  if (hasAny(tags, ['addition', 'subtraction'])) return { genreKey: 'addition-and-subtraction', label: '足し算と引き算' };
  if (hasAny(tags, ['multiplication', 'division'])) return { genreKey: 'multiplication-and-division', label: '掛け算と割り算' };
  return null;
}

export function derivedGradeTag(grade: ThemeDefinition['grade']): DerivedGradeTag | null {
  if (!grade) return null;
  const number = Number(grade.slug.slice('grade-'.length));
  if (number <= 6) return `grade_${number}` as DerivedGradeTag;
  return `junior_high_${number - 6}` as DerivedGradeTag;
}

export function taxonomyTags(theme: Pick<ThemeDefinition, 'grade' | 'tags'>): readonly ThemeTaxonomyTag[] {
  const gradeTag = derivedGradeTag(theme.grade);
  return gradeTag ? [...theme.tags, gradeTag] : theme.tags;
}

export function hasThemeTag(theme: Pick<ThemeDefinition, 'tags'>, tag: ThemeTag): boolean {
  return theme.tags.includes(tag);
}

export function defineTheme(input: ThemeDefinitionInput): ThemeDefinition {
  const uniqueTags = [...new Set(input.tags)];
  const gradeGenre = gradeGenreFromTags(uniqueTags);
  const recommendedGenre = recommendedGenreFromTags(uniqueTags);
  if (input.grade && !gradeGenre) throw new Error(`Theme ${input.themeKey} has no grade taxonomy genre.`);
  if (!recommendedGenre) throw new Error(`Theme ${input.themeKey} has no recommended taxonomy genre.`);
  return { ...input, tags: uniqueTags, gradeGenre, recommendedGenre };
}

export const ALL_MATH_STRUCTURES = ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'] as const satisfies readonly AnswerInputStructure[];
export const SIMPLE_POSITIVE: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: false };
export const SIMPLE_SIGNED: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: true };
export const SIMPLE_DECIMAL: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: true, allow_negative: false };
export const FRACTION_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction', 'mixed_fraction', 'decimal'] };
export const SIGNED_RATIONAL_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction', 'negative'] };
export const LINEAR_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ALL_MATH_STRUCTURES };
export const QUADRATIC_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction', 'root', 'negative', 'plus_minus', 'tuple', 'arithmetic'] };
export const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';
export const FRACTION_INSTRUCTION = '次の計算をしなさい。答えが仮分数になる場合は帯分数に直し、約分して最も簡単な形で答えなさい。';
export const IMPROPER_FRACTION_INSTRUCTION = '次の計算をしなさい。答えは仮分数のまま、約分して最も簡単な形で答えなさい。';

export function arithmeticTheme(base: Omit<ThemeDefinitionInput, 'promptKind' | 'answerSchemaKind' | 'worksheet'> & {
  answerSchemaKind?: 'integer' | 'rational' | 'decimal' | 'algebraic';
  title: string;
  instruction?: string;
  answerPlacement?: 'inline' | 'below';
}): ThemeDefinition {
  const { title, instruction = '', answerPlacement = 'inline', answerSchemaKind = 'integer', ...rest } = base;
  return defineTheme({ ...rest, promptKind: 'arithmetic', answerSchemaKind, worksheet: { title, instruction, answerPrefix: null, answerPlacement } });
}

export function columnArithmeticTheme(base: Omit<ThemeDefinitionInput, 'promptKind' | 'worksheet'> & {
  title: string;
  instruction?: string;
}): ThemeDefinition {
  const { title, instruction = '次の計算を筆算でしなさい。', ...rest } = base;
  return defineTheme({
    ...rest,
    promptKind: 'column_arithmetic',
    worksheet: { title, instruction, answerPrefix: null, answerPlacement: 'below' },
  });
}
