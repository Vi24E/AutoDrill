import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_GENERATOR_REVISION,
  ADDITION_LAYOUT,
  ADDITION_SKILL_ID,
  ADDITION_THEME_ID,
  LINEAR_EQUATION_1_CURRICULUM_PATH,
  LINEAR_EQUATION_1_GENERATOR_REVISION,
  LINEAR_EQUATION_1_LAYOUT,
  LINEAR_EQUATION_1_SKILL_ID,
  LINEAR_EQUATION_1_THEME_ID,
  LINEAR_EQUATION_2_CURRICULUM_PATH,
  LINEAR_EQUATION_2_GENERATOR_REVISION,
  LINEAR_EQUATION_2_LAYOUT,
  LINEAR_EQUATION_2_SKILL_ID,
  LINEAR_EQUATION_2_THEME_ID,
  type AnswerInputInterface,
  type AnswerInputStructure,
  type CurriculumPathSegment,
  type ProblemPrompt,
  type WorksheetLayout,
} from './drill-engine';

export type ThemePromptKind = ProblemPrompt['kind'];

export type ThemeDefinition = {
  numeric_theme_id: number;
  generator_revision: number;
  themeKey: string;
  label: string;
  grade: {
    slug: `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`;
    label: string;
  };
  gradeGenre: {
    genreKey: string;
    label: string;
  };
  recommendedGenre: {
    genreKey: string;
    label: string;
  } | null;
  problemCount: number;
  layout: WorksheetLayout;
  route: {
    gradeSlug: `grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}`;
    themeSlug: string;
    pathname: `/drills/grade-${1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9}/${string}`;
  };
  search: {
    title: string;
    description: string;
  };
  compatibility: {
    skillId: string;
    curriculumPath: readonly CurriculumPathSegment[];
  };
  promptKind: ThemePromptKind;
  answerSchemaKind: 'integer' | 'rational';
  inputInterface: AnswerInputInterface;
  worksheet: {
    title: string;
    instruction: string;
    answerPrefix: string | null;
  };
};

export const ALL_MATH_STRUCTURES = [
  'fraction',
  'mixed_fraction',
  'decimal',
  'root',
  'negative',
  'plus_minus',
  'tuple',
] as const satisfies readonly AnswerInputStructure[];

const LINEAR_INPUT_INTERFACE: AnswerInputInterface = {
  type: 'structured_math',
  allowed_structures: ALL_MATH_STRUCTURES,
};

const LINEAR_INSTRUCTION = '次の一次方程式を解きなさい。ただし、答えが整数でない場合は約分によって最も簡単な形の仮分数で答えなさい。';

export const ONE_DIGIT_ADDITION_DEFINITION: ThemeDefinition = {
  numeric_theme_id: ADDITION_THEME_ID,
  generator_revision: ADDITION_GENERATOR_REVISION,
  themeKey: 'jp.grade1.addition.one_digit',
  label: '一桁の足し算',
  grade: { slug: 'grade-1', label: '小学1年生' },
  gradeGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  recommendedGenre: { genreKey: 'addition-and-subtraction', label: '足し算と引き算' },
  problemCount: ADDITION_LAYOUT.problem_count,
  layout: ADDITION_LAYOUT,
  route: {
    gradeSlug: 'grade-1',
    themeSlug: 'one-digit-addition',
    pathname: '/drills/grade-1/one-digit-addition',
  },
  search: {
    title: '一桁の足し算 | AutoDrill',
    description: '小学1年生向けの一桁の足し算ドリルです。',
  },
  compatibility: {
    skillId: ADDITION_SKILL_ID,
    curriculumPath: ADDITION_CURRICULUM_PATH,
  },
  promptKind: 'addition',
  answerSchemaKind: 'integer',
  inputInterface: { type: 'simple_numeric', allow_decimal: false, allow_negative: false },
  worksheet: {
    title: '1けたのたしざん(1)',
    instruction: '',
    answerPrefix: null,
  },
};

export const LINEAR_EQUATION_1_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LINEAR_EQUATION_1_THEME_ID,
  generator_revision: LINEAR_EQUATION_1_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.1',
  label: '一次方程式(1)',
  grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' },
  recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: LINEAR_EQUATION_1_LAYOUT.problem_count,
  layout: LINEAR_EQUATION_1_LAYOUT,
  route: {
    gradeSlug: 'grade-7',
    themeSlug: 'linear-equation-1',
    pathname: '/drills/grade-7/linear-equation-1',
  },
  search: {
    title: '一次方程式(1) | AutoDrill',
    description: '中学1年生向けの整数解をもつ一次方程式ドリルです。',
  },
  compatibility: {
    skillId: LINEAR_EQUATION_1_SKILL_ID,
    curriculumPath: LINEAR_EQUATION_1_CURRICULUM_PATH,
  },
  promptKind: 'linear_equation',
  answerSchemaKind: 'integer',
  inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: {
    title: '一次方程式(1)',
    instruction: LINEAR_INSTRUCTION,
    answerPrefix: 'x =',
  },
};

export const LINEAR_EQUATION_2_DEFINITION: ThemeDefinition = {
  numeric_theme_id: LINEAR_EQUATION_2_THEME_ID,
  generator_revision: LINEAR_EQUATION_2_GENERATOR_REVISION,
  themeKey: 'jp.grade7.equation.linear.2',
  label: '一次方程式(2)',
  grade: { slug: 'grade-7', label: '中学1年生' },
  gradeGenre: { genreKey: 'linear-equation', label: '一次方程式' },
  recommendedGenre: { genreKey: 'equation', label: '方程式' },
  problemCount: LINEAR_EQUATION_2_LAYOUT.problem_count,
  layout: LINEAR_EQUATION_2_LAYOUT,
  route: {
    gradeSlug: 'grade-7',
    themeSlug: 'linear-equation-2',
    pathname: '/drills/grade-7/linear-equation-2',
  },
  search: {
    title: '一次方程式(2) | AutoDrill',
    description: '中学1年生向けの分数係数・分数解を含む一次方程式ドリルです。',
  },
  compatibility: {
    skillId: LINEAR_EQUATION_2_SKILL_ID,
    curriculumPath: LINEAR_EQUATION_2_CURRICULUM_PATH,
  },
  promptKind: 'linear_equation',
  answerSchemaKind: 'rational',
  inputInterface: LINEAR_INPUT_INTERFACE,
  worksheet: {
    title: '一次方程式(2)',
    instruction: LINEAR_INSTRUCTION,
    answerPrefix: 'x =',
  },
};

export const THEME_DEFINITIONS: readonly ThemeDefinition[] = [
  ONE_DIGIT_ADDITION_DEFINITION,
  LINEAR_EQUATION_1_DEFINITION,
  LINEAR_EQUATION_2_DEFINITION,
];

export function findThemeDefinitionByNumericId(numericThemeId: number): ThemeDefinition | undefined {
  return THEME_DEFINITIONS.find((theme) => theme.numeric_theme_id === numericThemeId);
}

export function sameInputInterface(left: AnswerInputInterface, right: AnswerInputInterface): boolean {
  if (left.type !== right.type) return false;
  if (left.type === 'simple_numeric' && right.type === 'simple_numeric') {
    return left.allow_decimal === right.allow_decimal && left.allow_negative === right.allow_negative;
  }
  if (left.type === 'structured_math' && right.type === 'structured_math') {
    return left.allowed_structures.length === right.allowed_structures.length
      && left.allowed_structures.every((structure, index) => structure === right.allowed_structures[index]);
  }
  return false;
}
