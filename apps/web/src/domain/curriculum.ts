import {
  DRILL_SCHEMA_VERSION,
  type DifficultyLevel,
} from '@/domain/drill-engine';
import { THEME_DEFINITIONS, type ThemeDefinition } from '@/domain/theme-registry';
import { LINEAR_EQUATION_1_DEFINITION } from '@/domain/themes/linear-equation-1';
import { LINEAR_EQUATION_2_DEFINITION } from '@/domain/themes/linear-equation-2';
import { ONE_DIGIT_ADDITION_DEFINITION } from '@/domain/themes/one-digit-addition';

export type { DifficultyLevel } from '@/domain/drill-engine';

export type CurriculumMode = 'recommended' | 'grade';
export type GradeNumber = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;
export type GradeSlug = `grade-${GradeNumber}`;

export const DIFFICULTY_OPTIONS = [
  { value: 1, label: 'かんたん' },
  { value: 2, label: 'ふつう' },
  { value: 3, label: 'むずかしい' },
  { value: 4, label: 'ランダム' },
] as const satisfies readonly { value: DifficultyLevel; label: string }[];

export type UnitRoute = ThemeDefinition['route'];

export type ImplementedCurriculumTheme = ThemeDefinition & {
  implemented: true;
};

export type UnimplementedCurriculumTheme = {
  numeric_theme_id: number;
  themeKey: string;
  label: string;
  implemented: false;
  generator_revision: null;
  problemCount: null;
  layout: null;
  route: null;
  search: null;
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

function implemented(definition: ThemeDefinition): ImplementedCurriculumTheme {
  return { ...definition, implemented: true };
}

export const IMPLEMENTED_THEMES: readonly ImplementedCurriculumTheme[] = THEME_DEFINITIONS.map(implemented);

export const ONE_DIGIT_ADDITION_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === ONE_DIGIT_ADDITION_DEFINITION.numeric_theme_id,
)!;

export const LINEAR_EQUATION_1_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_1_DEFINITION.numeric_theme_id,
)!;

export const LINEAR_EQUATION_2_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_2_DEFINITION.numeric_theme_id,
)!;

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

function implementedGenresForGrade(gradeSlug: GradeSlug): CurriculumGenre[] {
  const themes = IMPLEMENTED_THEMES.filter((theme) => theme.grade?.slug === gradeSlug);
  const groups = new Map<string, CurriculumGenre>();
  for (const theme of themes) {
    if (!theme.gradeGenre) continue;
    const key = theme.gradeGenre.genreKey;
    const existing = groups.get(key);
    if (existing) {
      groups.set(key, { ...existing, themes: [...existing.themes, theme] });
    } else {
      groups.set(key, { genreKey: key, label: theme.gradeGenre.label, themes: [theme] });
    }
  }
  return [...groups.values()];
}

export const CURRICULUM_TREE: readonly CurriculumGrade[] = GRADE_LABELS.map((label, index) => {
  const grade = (index + 1) as GradeNumber;
  const slug = `grade-${grade}` as GradeSlug;
  return {
    slug,
    label,
    genres: implementedGenresForGrade(slug),
  };
});

export const ADDITION_AND_SUBTRACTION_GENRE: CurriculumGenre = CURRICULUM_TREE[0]!.genres.find(
  (genre) => genre.genreKey === ONE_DIGIT_ADDITION_THEME.gradeGenre!.genreKey,
)!;

function buildRecommendedGenres(): CurriculumGenre[] {
  const groups = new Map<string, CurriculumGenre>();
  for (const theme of IMPLEMENTED_THEMES) {
    if (!theme.recommendedGenre) continue;
    const { genreKey, label } = theme.recommendedGenre;
    const existing = groups.get(genreKey);
    if (existing) {
      groups.set(genreKey, { ...existing, themes: [...existing.themes, theme] });
    } else {
      groups.set(genreKey, { genreKey, label, themes: [theme] });
    }
  }
  return [...groups.values()];
}

/** Recommended is a presentation grouping that references canonical theme objects. */
export const RECOMMENDED_GENRES: readonly CurriculumGenre[] = buildRecommendedGenres();

export const DEFAULT_WEB_DRILL_SETTINGS: WebDrillSettings = {
  schema_version: DRILL_SCHEMA_VERSION,
  numeric_theme_id: ONE_DIGIT_ADDITION_THEME.numeric_theme_id,
  themeKey: ONE_DIGIT_ADDITION_THEME.themeKey,
  difficulty: 2,
  seed: '',
};

export function createWebDrillSettings(
  theme: CurriculumTheme,
  difficulty: DifficultyLevel = 2,
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

  // Bonus themes intentionally live outside the grade tree. Keep the exact
  // selected theme while borrowing only the grade-mode navigation context;
  // Recommended mode resolves its genre directly from RECOMMENDED_GENRES.
  const bonusTheme = IMPLEMENTED_THEMES.find((theme) => theme.themeKey === themeKey && theme.grade === null);
  const grade = CURRICULUM_TREE.find((candidate) => candidate.slug === ONE_DIGIT_ADDITION_THEME.grade!.slug)!;
  const fallbackGenre = grade.genres.find((candidate) => candidate.genreKey === ONE_DIGIT_ADDITION_THEME.gradeGenre!.genreKey)!;
  if (bonusTheme) {
    const bonusGenre = RECOMMENDED_GENRES.find((candidate) => candidate.themes.some((theme) => theme.themeKey === themeKey)) ?? fallbackGenre;
    return { grade, genre: bonusGenre, theme: bonusTheme };
  }
  return { grade, genre: fallbackGenre, theme: ONE_DIGIT_ADDITION_THEME };
}

export function findTheme(themeKey: string): CurriculumTheme | undefined {
  return IMPLEMENTED_THEMES.find((theme) => theme.themeKey === themeKey)
    ?? CURRICULUM_TREE.flatMap((grade) => grade.genres)
      .flatMap((genre) => genre.themes)
      .find((theme) => theme.themeKey === themeKey);
}

export function findImplementedThemeByNumericId(numericThemeId: number): ImplementedCurriculumTheme | undefined {
  return IMPLEMENTED_THEMES.find((theme) => theme.numeric_theme_id === numericThemeId);
}

export function findImplementedThemeByRoute(
  gradeSlug: string,
  themeSlug: string,
): ImplementedCurriculumTheme | undefined {
  return IMPLEMENTED_THEMES.find((theme) => (
    theme.route.gradeSlug === gradeSlug && theme.route.themeSlug === themeSlug
  ));
}
