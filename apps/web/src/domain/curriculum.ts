import {
  DRILL_SCHEMA_VERSION,
  type DifficultyLevel,
} from '@/domain/drill-engine';
import { THEME_DEFINITIONS, type ThemeDefinition } from '@/domain/theme-registry';
import { LINEAR_EQUATION_SIMPLE_DEFINITION } from '@/domain/themes/linear-equation-simple';
import { LINEAR_EQUATION_1_DEFINITION } from '@/domain/themes/linear-equation-1';
import { LINEAR_EQUATION_2_DEFINITION } from '@/domain/themes/linear-equation-2';
import { LINEAR_EQUATION_3_DEFINITION } from '@/domain/themes/linear-equation-3';
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

export type CurriculumUnit = {
  unitKey: string;
  label: string;
  themes: readonly CurriculumTheme[];
};

export type CurriculumGenre = {
  genreKey: string;
  label: string;
  themes: readonly CurriculumTheme[];
};

export type CurriculumGrade = {
  slug: GradeSlug;
  label: string;
  units: readonly CurriculumUnit[];
};

export type CurriculumSelection = {
  grade: CurriculumGrade;
  unit: CurriculumUnit;
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

export const LINEAR_EQUATION_SIMPLE_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_SIMPLE_DEFINITION.numeric_theme_id,
)!;

export const LINEAR_EQUATION_1_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_1_DEFINITION.numeric_theme_id,
)!;

export const LINEAR_EQUATION_2_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_2_DEFINITION.numeric_theme_id,
)!;

export const LINEAR_EQUATION_3_THEME = IMPLEMENTED_THEMES.find(
  (theme) => theme.numeric_theme_id === LINEAR_EQUATION_3_DEFINITION.numeric_theme_id,
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

function implementedUnitsForGrade(gradeSlug: GradeSlug): CurriculumUnit[] {
  const themes = IMPLEMENTED_THEMES.filter((theme) => theme.grade?.slug === gradeSlug);
  const units = new Map<string, CurriculumUnit>();
  for (const theme of themes) {
    if (!theme.curriculumUnit) continue;
    const { unitKey, label } = theme.curriculumUnit;
    const existing = units.get(unitKey);
    if (existing) {
      units.set(unitKey, { ...existing, themes: [...existing.themes, theme] });
    } else {
      units.set(unitKey, { unitKey, label, themes: [theme] });
    }
  }
  return [...units.values()];
}

export const CURRICULUM_TREE: readonly CurriculumGrade[] = GRADE_LABELS.map((label, index) => {
  const grade = (index + 1) as GradeNumber;
  const slug = `grade-${grade}` as GradeSlug;
  return {
    slug,
    label,
    units: implementedUnitsForGrade(slug),
  };
});

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

/** Recommended is a presentation grouping independent of grade curriculum units. */
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
    for (const unit of grade.units) {
      const theme = unit.themes.find((candidate) => candidate.themeKey === themeKey);
      if (theme) return { grade, unit, theme };
    }
  }

  // Bonus themes intentionally live outside the grade tree. Keep the exact
  // selected theme while borrowing only the grade-mode navigation context;
  // Recommended mode resolves its genre directly from RECOMMENDED_GENRES.
  const bonusTheme = IMPLEMENTED_THEMES.find((theme) => theme.themeKey === themeKey && theme.grade === null);
  const grade = CURRICULUM_TREE.find((candidate) => candidate.slug === ONE_DIGIT_ADDITION_THEME.grade!.slug)!;
  const fallbackUnit = grade.units.find((candidate) => candidate.unitKey === ONE_DIGIT_ADDITION_THEME.curriculumUnit!.unitKey)!;
  if (bonusTheme) return { grade, unit: fallbackUnit, theme: bonusTheme };
  return { grade, unit: fallbackUnit, theme: ONE_DIGIT_ADDITION_THEME };
}

export function findTheme(themeKey: string): CurriculumTheme | undefined {
  return IMPLEMENTED_THEMES.find((theme) => theme.themeKey === themeKey)
    ?? CURRICULUM_TREE.flatMap((grade) => grade.units)
      .flatMap((unit) => unit.themes)
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
