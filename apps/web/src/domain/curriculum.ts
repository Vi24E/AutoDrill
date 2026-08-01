import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_LAYOUT,
  ADDITION_SKILL_ID,
  DRILL_SCHEMA_VERSION,
  type DifficultyLevel,
  type CurriculumPathSegment,
  type WorksheetLayout,
} from '@/domain/drill-engine';

export type { DifficultyLevel } from '@/domain/drill-engine';

export type CurriculumMode = 'recommended' | 'grade';
export type GradeNumber = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
export type GradeSlug = `grade-${GradeNumber}`;

export const DIFFICULTY_OPTIONS = [
  { value: 1, label: '1: とてもやさしい' },
  { value: 2, label: '2: やさしい' },
  { value: 3, label: '3: ふつう' },
  { value: 4, label: '4: むずかしい' },
  { value: 5, label: '5: とてもむずかしい' },
] as const satisfies readonly { value: DifficultyLevel; label: string }[];

export type UnitRoute = {
  gradeSlug: GradeSlug;
  themeSlug: string;
  pathname: `/drills/${GradeSlug}/${string}`;
};

type ThemeBase = {
  numeric_theme_id: number;
  themeKey: string;
  label: string;
};

export type ImplementedCurriculumTheme = ThemeBase & {
  implemented: true;
  numeric_theme_id: 1;
  themeKey: 'jp.grade1.addition.one_digit';
  generator_revision: 2;
  problemCount: 20;
  layout: WorksheetLayout;
  route: UnitRoute;
  search: {
    title: string;
    description: string;
  };
  compatibility: {
    skillId: typeof ADDITION_SKILL_ID;
    curriculumPath: readonly CurriculumPathSegment[];
  };
};

export type UnimplementedCurriculumTheme = ThemeBase & {
  implemented: false;
  generator_revision: null;
  problemCount: null;
  layout: null;
  route: null;
  search: null;
  compatibility: null;
};

export type CurriculumTheme = ImplementedCurriculumTheme | UnimplementedCurriculumTheme;

export type CurriculumGenre = {
  genreKey: string;
  label: string;
  themes: readonly CurriculumTheme[];
};

export type CurriculumGrade = {
  slug: GradeSlug;
  label: string;
  genres: readonly CurriculumGenre[];
};

export type CurriculumSelection = {
  grade: CurriculumGrade;
  genre: CurriculumGenre;
  theme: CurriculumTheme;
};

export type WebDrillSettings = {
  schema_version: typeof DRILL_SCHEMA_VERSION;
  numeric_theme_id: number;
  themeKey: string;
  difficulty: DifficultyLevel;
  seed: string;
};

export const ONE_DIGIT_ADDITION_THEME: ImplementedCurriculumTheme = {
  numeric_theme_id: 1,
  themeKey: 'jp.grade1.addition.one_digit',
  label: '一桁の足し算',
  implemented: true,
  generator_revision: 2,
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
};

export const ADDITION_AND_SUBTRACTION_GENRE: CurriculumGenre = {
  genreKey: 'addition-and-subtraction',
  label: '足し算と引き算',
  themes: [ONE_DIGIT_ADDITION_THEME],
};

const GRADE_LABELS = [
  '小学1年生',
  '小学2年生',
  '小学3年生',
  '小学4年生',
  '小学5年生',
  '小学6年生',
  '中学1年生',
  '中学2年生',
  '中学3年生',
] as const;

function createDummyGenre(grade: GradeNumber): CurriculumGenre {
  const theme: UnimplementedCurriculumTheme = {
    numeric_theme_id: 100 + grade,
    themeKey: `dummy.grade-${grade}.theme-1`,
    label: 'Dummy1',
    implemented: false,
    generator_revision: null,
    problemCount: null,
    layout: null,
    route: null,
    search: null,
    compatibility: null,
  };
  return {
    genreKey: `dummy-grade-${grade}-genre-1`,
    label: 'Dummy1',
    themes: [theme],
  };
}

export const CURRICULUM_TREE: readonly CurriculumGrade[] = GRADE_LABELS.map((label, index) => {
  const grade = (index + 1) as GradeNumber;
  return {
    slug: `grade-${grade}` as GradeSlug,
    label,
    genres: grade === 1
      ? [ADDITION_AND_SUBTRACTION_GENRE, createDummyGenre(grade)]
      : [createDummyGenre(grade)],
  };
});

/** Recommended is a reference-only subset of the canonical grade tree. */
export const RECOMMENDED_GENRES: readonly CurriculumGenre[] = [CURRICULUM_TREE[0]!.genres[0]!];

export const IMPLEMENTED_THEMES: readonly ImplementedCurriculumTheme[] = [ONE_DIGIT_ADDITION_THEME];

export const DEFAULT_WEB_DRILL_SETTINGS: WebDrillSettings = {
  schema_version: DRILL_SCHEMA_VERSION,
  numeric_theme_id: ONE_DIGIT_ADDITION_THEME.numeric_theme_id,
  themeKey: ONE_DIGIT_ADDITION_THEME.themeKey,
  difficulty: 3,
  seed: '',
};

export function createWebDrillSettings(
  theme: CurriculumTheme,
  difficulty: DifficultyLevel = 3,
  seed = '',
): WebDrillSettings {
  return {
    schema_version: DRILL_SCHEMA_VERSION,
    numeric_theme_id: theme.numeric_theme_id,
    themeKey: theme.themeKey,
    difficulty,
    seed,
  };
}

export function findCurriculumSelection(themeKey: string): CurriculumSelection {
  for (const grade of CURRICULUM_TREE) {
    for (const genre of grade.genres) {
      const theme = genre.themes.find((candidate) => candidate.themeKey === themeKey);
      if (theme) return { grade, genre, theme };
    }
  }

  const grade = CURRICULUM_TREE[0]!;
  const genre = grade.genres[0]!;
  return { grade, genre, theme: genre.themes[0]! };
}

export function findTheme(themeKey: string): CurriculumTheme | undefined {
  for (const grade of CURRICULUM_TREE) {
    for (const genre of grade.genres) {
      const theme = genre.themes.find((candidate) => candidate.themeKey === themeKey);
      if (theme) return theme;
    }
  }
  return undefined;
}

export function findImplementedThemeByRoute(
  gradeSlug: string,
  themeSlug: string,
): ImplementedCurriculumTheme | undefined {
  return IMPLEMENTED_THEMES.find((theme) => (
    theme.route.gradeSlug === gradeSlug && theme.route.themeSlug === themeSlug
  ));
}
