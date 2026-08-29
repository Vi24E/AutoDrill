import type { AnswerInputInterface, CurriculumPathSegment, ProblemPrompt, WorksheetLayout } from '../drill-engine';
import { DRILL_CORE_CONTRACT } from '@/generated/drill-core-contract';

export type ThemePromptKind = ProblemPrompt['kind'];
export type NumericThemeId = typeof DRILL_CORE_CONTRACT.themes[keyof typeof DRILL_CORE_CONTRACT.themes]['numeric_theme_id'];
export type GradeNumber = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
export type GradeSlug = `grade-${GradeNumber}`;
export type RouteGroupSlug = GradeSlug | 'bonus';

type CoreThemeContract = typeof DRILL_CORE_CONTRACT.themes[keyof typeof DRILL_CORE_CONTRACT.themes];
export type ThemeTag = CoreThemeContract['tags'][number];
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
  grade: { number: GradeNumber; slug: GradeSlug; label: string } | null;
  tags: readonly ThemeTag[];
  /** Derived by defineTheme; retained as a stable read-only projection for callers. */
  gradeGenre: ThemeGenreMetadata | null;
  /** Derived by defineTheme; retained as a stable read-only projection for callers. */
  recommendedGenre: ThemeGenreMetadata | null;
  problemCount: number;
  layout: WorksheetLayout;
  route: { gradeSlug: RouteGroupSlug; themeSlug: string; pathname: `/drills/${RouteGroupSlug}/${string}` };
  search: { title: string; description: string };
  curriculumPath: readonly CurriculumPathSegment[];
  safety: 'non_negative_only' | 'unrestricted';
  presentation: {
    worksheet_grid: boolean;
    column_arithmetic: boolean;
    print_recommended: boolean;
    equation_layout: boolean;
    fraction: 'none' | 'mixed_number_when_improper' | 'keep_improper_fraction';
    column_input: {
      single: { order: 'least_significant_first' | 'natural_division_flow' | 'big_endian'; decimal_point: 'none' | 'fixed_canonical_scale' | 'editable' } | null;
      quotient: { order: 'least_significant_first' | 'natural_division_flow' | 'big_endian'; decimal_point: 'none' | 'fixed_canonical_scale' | 'editable' } | null;
      remainder: { order: 'least_significant_first' | 'natural_division_flow' | 'big_endian'; decimal_point: 'none' | 'fixed_canonical_scale' | 'editable' } | null;
    } | null;
  };
  dedup: 'canonicalize_commutative' | 'preserve_operand_order';
  promptKind: ThemePromptKind;
  answerSchemaKind: 'integer' | 'rational' | 'decimal' | 'ordered_pair' | 'ordered_tuple' | 'algebraic';
  inputInterface: AnswerInputInterface;
  editorInputInterface: AnswerInputInterface;
  worksheet: { title: string; instruction: string; answerPrefix: string | null; answerPlacement?: 'inline' | 'below' };
};

/**
 * `numeric_theme_id` is the only hand-written Rust foreign key in a Web theme
 * definition. `defineTheme` resolves and validates every other cross-language
 * field (revision, skill id, curriculum, layout, capabilities) from generated Rust metadata.
 */
export type ThemeDefinitionInput = Omit<
  ThemeDefinition,
  'numeric_theme_id' | 'generator_revision' | 'themeKey' | 'grade' | 'tags' | 'gradeGenre' | 'recommendedGenre' | 'problemCount' | 'layout' | 'route' | 'curriculumPath' | 'safety' | 'presentation' | 'dedup' | 'promptKind' | 'answerSchemaKind' | 'inputInterface' | 'editorInputInterface'
> & {
  numeric_theme_id: NumericThemeId;
  route: { themeSlug: string };
};

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
  const number = grade.number;
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
  const core = DRILL_CORE_CONTRACT.themes[String(input.numeric_theme_id) as keyof typeof DRILL_CORE_CONTRACT.themes];
  if (!core || core.numeric_theme_id !== input.numeric_theme_id) {
    throw new Error(`Theme ${input.numeric_theme_id} has no Rust theme contract.`);
  }
  const grade = core.grade === null
    ? null
    : { number: core.grade as GradeNumber, slug: `grade-${core.grade}` as GradeSlug, label: core.curriculum_path[1] ?? `Grade ${core.grade}` };
  const routeGroup = grade?.slug ?? 'bonus';
  const uniqueTags = [...new Set(core.tags)] as ThemeTag[];
  const gradeGenre = gradeGenreFromTags(uniqueTags);
  const recommendedGenre = recommendedGenreFromTags(uniqueTags);
  if (grade && !gradeGenre) throw new Error(`Theme ${core.skill_id} has no grade taxonomy genre.`);
  if (!recommendedGenre) throw new Error(`Theme ${core.skill_id} has no recommended taxonomy genre.`);
  const curriculumPath = core.curriculum_path.map((label, index) => ({
    id: index === 0 ? 'root' : index === core.curriculum_path.length - 1 ? core.skill_id : `${core.skill_id}:path:${index}`,
    label,
  }));
  return {
    ...input,
    themeKey: core.skill_id,
    route: {
      gradeSlug: routeGroup,
      themeSlug: input.route.themeSlug,
      pathname: `/drills/${routeGroup}/${input.route.themeSlug}` as `/drills/${RouteGroupSlug}/${string}`,
    },
    generator_revision: core.generator_revision,
    grade,
    tags: uniqueTags,
    gradeGenre,
    recommendedGenre,
    problemCount: core.layout.problem_count,
    layout: core.layout,
    curriculumPath,
    safety: core.safety,
    presentation: core.presentation,
    dedup: core.dedup,
    promptKind: core.answer_contract.prompt_kind as ThemePromptKind,
    answerSchemaKind: core.answer_contract.answer_schema_kind,
    inputInterface: core.input_interface as AnswerInputInterface,
    editorInputInterface: core.editor_input_interface as AnswerInputInterface,
  };
}

export const ALL_MATH_STRUCTURES = DRILL_CORE_CONTRACT.editor_structures;
export const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';
export const FRACTION_INSTRUCTION = '次の計算をしなさい。答えが仮分数になる場合は帯分数に直し、約分して最も簡単な形で答えなさい。';
export const IMPROPER_FRACTION_INSTRUCTION = '次の計算をしなさい。答えは仮分数のまま、約分して最も簡単な形で答えなさい。';

export function arithmeticTheme(base: Omit<ThemeDefinitionInput, 'worksheet'> & {
  title: string;
  instruction?: string;
  answerPlacement?: 'inline' | 'below';
}): ThemeDefinition {
  const { title, instruction = '', answerPlacement = 'inline', ...rest } = base;
  return defineTheme({ ...rest, worksheet: { title, instruction, answerPrefix: null, answerPlacement } });
}

export function columnArithmeticTheme(base: Omit<ThemeDefinitionInput, 'worksheet'> & {
  title: string;
  instruction?: string;
}): ThemeDefinition {
  const { title, instruction = '次の計算を筆算でしなさい。', ...rest } = base;
  return defineTheme({
    ...rest,
    worksheet: { title, instruction, answerPrefix: null, answerPlacement: 'below' },
  });
}
