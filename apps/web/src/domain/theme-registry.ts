import {
  ADDITION_CURRICULUM_PATH, ADDITION_GENERATOR_REVISION, ADDITION_LAYOUT, ADDITION_SKILL_ID, ADDITION_THEME_ID,
  FRACTION_ADDITION_CURRICULUM_PATH, FRACTION_ADDITION_GENERATOR_REVISION, FRACTION_ADDITION_LAYOUT, FRACTION_ADDITION_SKILL_ID, FRACTION_ADDITION_THEME_ID,
  FRACTION_SUBTRACTION_CURRICULUM_PATH, FRACTION_SUBTRACTION_GENERATOR_REVISION, FRACTION_SUBTRACTION_LAYOUT, FRACTION_SUBTRACTION_SKILL_ID, FRACTION_SUBTRACTION_THEME_ID,
  FRACTION_MULTIPLICATION_CURRICULUM_PATH, FRACTION_MULTIPLICATION_GENERATOR_REVISION, FRACTION_MULTIPLICATION_LAYOUT, FRACTION_MULTIPLICATION_SKILL_ID, FRACTION_MULTIPLICATION_THEME_ID,
  LINEAR_EQUATION_1_CURRICULUM_PATH, LINEAR_EQUATION_1_GENERATOR_REVISION, LINEAR_EQUATION_1_LAYOUT, LINEAR_EQUATION_1_SKILL_ID, LINEAR_EQUATION_1_THEME_ID,
  LINEAR_EQUATION_2_CURRICULUM_PATH, LINEAR_EQUATION_2_GENERATOR_REVISION, LINEAR_EQUATION_2_LAYOUT, LINEAR_EQUATION_2_SKILL_ID, LINEAR_EQUATION_2_THEME_ID,
  MULTIPLICATION_TABLE_CURRICULUM_PATH, MULTIPLICATION_TABLE_GENERATOR_REVISION, MULTIPLICATION_TABLE_LAYOUT, MULTIPLICATION_TABLE_SKILL_ID, MULTIPLICATION_TABLE_THEME_ID,
  ONE_DIGIT_SUBTRACTION_CURRICULUM_PATH, ONE_DIGIT_SUBTRACTION_GENERATOR_REVISION, ONE_DIGIT_SUBTRACTION_LAYOUT, ONE_DIGIT_SUBTRACTION_SKILL_ID, ONE_DIGIT_SUBTRACTION_THEME_ID,
  SIGNED_ARITHMETIC_1_CURRICULUM_PATH, SIGNED_ARITHMETIC_1_GENERATOR_REVISION, SIGNED_ARITHMETIC_1_LAYOUT, SIGNED_ARITHMETIC_1_SKILL_ID, SIGNED_ARITHMETIC_1_THEME_ID,
  SIGNED_ARITHMETIC_2_CURRICULUM_PATH, SIGNED_ARITHMETIC_2_GENERATOR_REVISION, SIGNED_ARITHMETIC_2_LAYOUT, SIGNED_ARITHMETIC_2_SKILL_ID, SIGNED_ARITHMETIC_2_THEME_ID,
  TWO_DIGIT_ADDITION_CURRICULUM_PATH, TWO_DIGIT_ADDITION_GENERATOR_REVISION, TWO_DIGIT_ADDITION_LAYOUT, TWO_DIGIT_ADDITION_SKILL_ID, TWO_DIGIT_ADDITION_THEME_ID,
  type AnswerInputInterface, type AnswerInputStructure, type CurriculumPathSegment, type ProblemPrompt, type WorksheetLayout,
} from './drill-engine';

export type ThemePromptKind = ProblemPrompt['kind'];
export type ThemeDefinition = {
  numeric_theme_id: number;
  generator_revision: number;
  themeKey: string;
  label: string;
  grade: { slug: `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`; label: string };
  gradeGenre: { genreKey: string; label: string };
  recommendedGenre: { genreKey: string; label: string } | null;
  problemCount: number;
  layout: WorksheetLayout;
  route: { gradeSlug: `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`; themeSlug: string; pathname: `/drills/grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}/${string}` };
  search: { title: string; description: string };
  compatibility: { skillId: string; curriculumPath: readonly CurriculumPathSegment[] };
  promptKind: ThemePromptKind;
  answerSchemaKind: 'integer' | 'rational';
  inputInterface: AnswerInputInterface;
  worksheet: { title: string; instruction: string; answerPrefix: string | null };
};

export const ALL_MATH_STRUCTURES = ['fraction', 'mixed_fraction', 'decimal', 'root', 'negative', 'plus_minus', 'tuple'] as const satisfies readonly AnswerInputStructure[];
const SIMPLE_POSITIVE: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: false };
const SIMPLE_SIGNED: AnswerInputInterface = { type: 'simple_numeric', allow_decimal: false, allow_negative: true };
const FRACTION_INPUT: AnswerInputInterface = { type: 'structured_math', allowed_structures: ['fraction'] };
const LINEAR_INPUT_INTERFACE: AnswerInputInterface = { type: 'structured_math', allowed_structures: ALL_MATH_STRUCTURES };
const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';
const FRACTION_INSTRUCTION = '次の計算をしなさい。答えは約分して最も簡単な分数で答えなさい。';

function arithmeticTheme(base: Omit<ThemeDefinition, 'promptKind' | 'answerSchemaKind' | 'worksheet'> & {
  answerSchemaKind?: 'integer' | 'rational';
  title: string;
  instruction?: string;
}): ThemeDefinition {
  const { title, instruction = '', answerSchemaKind = 'integer', ...rest } = base;
  return { ...rest, promptKind: 'arithmetic', answerSchemaKind, worksheet: { title, instruction, answerPrefix: null } };
}

export const ONE_DIGIT_ADDITION_DEFINITION: ThemeDefinition = {
  numeric_theme_id: ADDITION_THEME_ID, generator_revision: ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade1.addition.one_digit', label: '一桁の足し算', grade: { slug: 'grade-1', label: '小学1年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' }, recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: ADDITION_LAYOUT.problem_count, layout: ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-1', themeSlug: 'one-digit-addition', pathname: '/drills/grade-1/one-digit-addition' },
  search: { title: '一桁の足し算 | AutoDrill', description: '小学1年生向けの一桁の足し算ドリルです。' },
  compatibility: { skillId: ADDITION_SKILL_ID, curriculumPath: ADDITION_CURRICULUM_PATH }, promptKind: 'addition', answerSchemaKind: 'integer', inputInterface: SIMPLE_POSITIVE,
  worksheet: { title: '1けたのたしざん(1)', instruction: '', answerPrefix: null },
};

export const ONE_DIGIT_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: ONE_DIGIT_SUBTRACTION_THEME_ID, generator_revision: ONE_DIGIT_SUBTRACTION_GENERATOR_REVISION,
  themeKey: 'jp.grade1.subtraction.one_digit', label: '一桁の引き算', grade: { slug: 'grade-1', label: '小学1年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' }, recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: ONE_DIGIT_SUBTRACTION_LAYOUT.problem_count, layout: ONE_DIGIT_SUBTRACTION_LAYOUT,
  route: { gradeSlug: 'grade-1', themeSlug: 'one-digit-subtraction', pathname: '/drills/grade-1/one-digit-subtraction' },
  search: { title: '一桁の引き算 | AutoDrill', description: '小学1年生向けの一桁の引き算ドリルです。' },
  compatibility: { skillId: ONE_DIGIT_SUBTRACTION_SKILL_ID, curriculumPath: ONE_DIGIT_SUBTRACTION_CURRICULUM_PATH }, inputInterface: SIMPLE_POSITIVE, title: '1けたのひきざん',
});

export const TWO_DIGIT_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: TWO_DIGIT_ADDITION_THEME_ID, generator_revision: TWO_DIGIT_ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade2.addition.two_digit', label: '二桁の足し算', grade: { slug: 'grade-2', label: '小学2年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' }, recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: TWO_DIGIT_ADDITION_LAYOUT.problem_count, layout: TWO_DIGIT_ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'two-digit-addition', pathname: '/drills/grade-2/two-digit-addition' },
  search: { title: '二桁の足し算 | AutoDrill', description: '小学2年生向けの二桁の足し算ドリルです。' },
  compatibility: { skillId: TWO_DIGIT_ADDITION_SKILL_ID, curriculumPath: TWO_DIGIT_ADDITION_CURRICULUM_PATH }, inputInterface: SIMPLE_POSITIVE, title: '2けたのたしざん',
});

export const MULTIPLICATION_TABLE_DEFINITION = arithmeticTheme({
  numeric_theme_id: MULTIPLICATION_TABLE_THEME_ID, generator_revision: MULTIPLICATION_TABLE_GENERATOR_REVISION,
  themeKey: 'jp.grade2.multiplication.table', label: '九九', grade: { slug: 'grade-2', label: '小学2年生' },
  gradeGenre: { genreKey: 'multiplication-and-division', label: '掛け算と割り算' }, recommendedGenre: { genreKey: 'multiplication-and-division', label: '掛け算と割り算' },
  problemCount: MULTIPLICATION_TABLE_LAYOUT.problem_count, layout: MULTIPLICATION_TABLE_LAYOUT,
  route: { gradeSlug: 'grade-2', themeSlug: 'multiplication-table', pathname: '/drills/grade-2/multiplication-table' },
  search: { title: '九九 | AutoDrill', description: '小学2年生向けの九九ドリルです。' },
  compatibility: { skillId: MULTIPLICATION_TABLE_SKILL_ID, curriculumPath: MULTIPLICATION_TABLE_CURRICULUM_PATH }, inputInterface: SIMPLE_POSITIVE, title: '九九',
});

export const FRACTION_ADDITION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_ADDITION_THEME_ID, generator_revision: FRACTION_ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.fraction.addition', label: '分数の足し算', grade: { slug: 'grade-5', label: '小学5年生' },
  gradeGenre: { genreKey: 'fractions', label: '分数' }, recommendedGenre: { genreKey: 'fractions', label: '分数' },
  problemCount: FRACTION_ADDITION_LAYOUT.problem_count, layout: FRACTION_ADDITION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'fraction-addition', pathname: '/drills/grade-5/fraction-addition' },
  search: { title: '分数の足し算 | AutoDrill', description: '小学5年生向けの分数の足し算ドリルです。' },
  compatibility: { skillId: FRACTION_ADDITION_SKILL_ID, curriculumPath: FRACTION_ADDITION_CURRICULUM_PATH }, inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の足し算', instruction: FRACTION_INSTRUCTION,
});

export const FRACTION_SUBTRACTION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_SUBTRACTION_THEME_ID, generator_revision: FRACTION_SUBTRACTION_GENERATOR_REVISION,
  themeKey: 'jp.grade5.fraction.subtraction', label: '分数の引き算', grade: { slug: 'grade-5', label: '小学5年生' },
  gradeGenre: { genreKey: 'fractions', label: '分数' }, recommendedGenre: { genreKey: 'fractions', label: '分数' },
  problemCount: FRACTION_SUBTRACTION_LAYOUT.problem_count, layout: FRACTION_SUBTRACTION_LAYOUT,
  route: { gradeSlug: 'grade-5', themeSlug: 'fraction-subtraction', pathname: '/drills/grade-5/fraction-subtraction' },
  search: { title: '分数の引き算 | AutoDrill', description: '小学5年生向けの正の分数の引き算ドリルです。' },
  compatibility: { skillId: FRACTION_SUBTRACTION_SKILL_ID, curriculumPath: FRACTION_SUBTRACTION_CURRICULUM_PATH }, inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の引き算', instruction: FRACTION_INSTRUCTION,
});

export const FRACTION_MULTIPLICATION_DEFINITION = arithmeticTheme({
  numeric_theme_id: FRACTION_MULTIPLICATION_THEME_ID, generator_revision: FRACTION_MULTIPLICATION_GENERATOR_REVISION,
  themeKey: 'jp.grade6.fraction.multiplication', label: '分数の掛け算', grade: { slug: 'grade-6', label: '小学6年生' },
  gradeGenre: { genreKey: 'fractions', label: '分数' }, recommendedGenre: { genreKey: 'fractions', label: '分数' },
  problemCount: FRACTION_MULTIPLICATION_LAYOUT.problem_count, layout: FRACTION_MULTIPLICATION_LAYOUT,
  route: { gradeSlug: 'grade-6', themeSlug: 'fraction-multiplication', pathname: '/drills/grade-6/fraction-multiplication' },
  search: { title: '分数の掛け算 | AutoDrill', description: '小学6年生向けの分数の掛け算ドリルです。' },
  compatibility: { skillId: FRACTION_MULTIPLICATION_SKILL_ID, curriculumPath: FRACTION_MULTIPLICATION_CURRICULUM_PATH }, inputInterface: FRACTION_INPUT, answerSchemaKind: 'rational', title: '分数の掛け算', instruction: FRACTION_INSTRUCTION,
});

export const SIGNED_ARITHMETIC_1_DEFINITION = arithmeticTheme({
  numeric_theme_id: SIGNED_ARITHMETIC_1_THEME_ID, generator_revision: SIGNED_ARITHMETIC_1_GENERATOR_REVISION,
  themeKey: 'jp.grade7.signed.arithmetic.1', label: '負の数の計算(1)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'signed-numbers', label: '正の数・負の数' }, recommendedGenre: { genreKey: 'negative-numbers', label: '負の数' },
  problemCount: SIGNED_ARITHMETIC_1_LAYOUT.problem_count, layout: SIGNED_ARITHMETIC_1_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-1', pathname: '/drills/grade-7/signed-arithmetic-1' },
  search: { title: '負の数の計算(1) | AutoDrill', description: '中学1年生向けの正負の整数の加減ドリルです。' },
  compatibility: { skillId: SIGNED_ARITHMETIC_1_SKILL_ID, curriculumPath: SIGNED_ARITHMETIC_1_CURRICULUM_PATH }, inputInterface: SIMPLE_SIGNED, title: '負の数の計算(1)', instruction: '次の計算をしなさい。',
});

export const SIGNED_ARITHMETIC_2_DEFINITION = arithmeticTheme({
  numeric_theme_id: SIGNED_ARITHMETIC_2_THEME_ID, generator_revision: SIGNED_ARITHMETIC_2_GENERATOR_REVISION,
  themeKey: 'jp.grade7.signed.arithmetic.2', label: '負の数の計算(2)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'signed-numbers', label: '正の数・負の数' }, recommendedGenre: { genreKey: 'negative-numbers', label: '負の数' },
  problemCount: SIGNED_ARITHMETIC_2_LAYOUT.problem_count, layout: SIGNED_ARITHMETIC_2_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'signed-arithmetic-2', pathname: '/drills/grade-7/signed-arithmetic-2' },
  search: { title: '負の数の計算(2) | AutoDrill', description: '中学1年生向けの正負の整数の四則計算ドリルです。' },
  compatibility: { skillId: SIGNED_ARITHMETIC_2_SKILL_ID, curriculumPath: SIGNED_ARITHMETIC_2_CURRICULUM_PATH }, inputInterface: SIMPLE_SIGNED, title: '負の数の計算(2)', instruction: '次の計算をしなさい。',
});

export const LINEAR_EQUATION_1_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LINEAR_EQUATION_1_THEME_ID, generator_revision: LINEAR_EQUATION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.1', label: '一次方程式(1)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' }, recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: LINEAR_EQUATION_1_LAYOUT.problem_count, layout: LINEAR_EQUATION_1_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-1', pathname: '/drills/grade-7/linear-equation-1' },
  search: { title: '一次方程式(1) | AutoDrill', description: '中学1年生向けの整数解をもつ一次方程式ドリルです。' },
  compatibility: { skillId: LINEAR_EQUATION_1_SKILL_ID, curriculumPath: LINEAR_EQUATION_1_CURRICULUM_PATH }, promptKind: 'linear_equation', answerSchemaKind: 'integer', inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: { title: '一次方程式(1)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
};
export const LINEAR_EQUATION_2_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LINEAR_EQUATION_2_THEME_ID, generator_revision: LINEAR_EQUATION_2_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.2', label: '一次方程式(2)', grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' }, recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: LINEAR_EQUATION_2_LAYOUT.problem_count, layout: LINEAR_EQUATION_2_LAYOUT,
  route: { gradeSlug: 'grade-7', themeSlug: 'linear-equation-2', pathname: '/drills/grade-7/linear-equation-2' },
  search: { title: '一次方程式(2) | AutoDrill', description: '中学1年生向けの分数係数・分数解を含む一次方程式ドリルです。' },
  compatibility: { skillId: LINEAR_EQUATION_2_SKILL_ID, curriculumPath: LINEAR_EQUATION_2_CURRICULUM_PATH }, promptKind: 'linear_equation', answerSchemaKind: 'rational', inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: { title: '一次方程式(2)', instruction: LINEAR_INSTRUCTION, answerPrefix: 'x =' },
};

export const THEME_DEFINITIONS: readonly ThemeDefinition[] = [
  ONE_DIGIT_ADDITION_DEFINITION, ONE_DIGIT_SUBTRACTION_DEFINITION, TWO_DIGIT_ADDITION_DEFINITION,
  MULTIPLICATION_TABLE_DEFINITION, FRACTION_ADDITION_DEFINITION, FRACTION_SUBTRACTION_DEFINITION, FRACTION_MULTIPLICATION_DEFINITION,
  SIGNED_ARITHMETIC_1_DEFINITION, SIGNED_ARITHMETIC_2_DEFINITION,
  LINEAR_EQUATION_1_DEFINITION, LINEAR_EQUATION_2_DEFINITION,
];

export function findThemeDefinitionByNumericId(numericThemeId: number): ThemeDefinition | undefined {
  return THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === numericThemeId);
}
export function sameInputInterface(left: AnswerInputInterface, right: AnswerInputInterface): boolean {
  if (left.type !== right.type) return false;
  if (left.type === 'simple_numeric' && right.type === 'simple_numeric') return left.allow_decimal === right.allow_decimal && left.allow_negative === right.allow_negative;
  if (left.type === 'structured_math' && right.type === 'structured_math') return left.allowed_structures.length === right.allowed_structures.length && left.allowed_structures.every((structure, index) => structure === right.allowed_structures[index]);
  return false;
}
