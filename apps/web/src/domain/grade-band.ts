export type WorksheetGradeBand = 'early-elementary' | 'late-elementary' | 'junior-high';

/** Shared typography band for Web and print worksheets. Grades 7–9 are junior high. */
export function worksheetGradeBand(gradeSlug: string): WorksheetGradeBand {
  const match = /^grade-(\d+)$/.exec(gradeSlug);
  const grade = match ? Number(match[1]) : 1;
  if (grade >= 7) return 'junior-high';
  if (grade >= 4) return 'late-elementary';
  return 'early-elementary';
}

export function worksheetGradeBandClass(gradeSlug: string): string {
  return `worksheet-grade-${worksheetGradeBand(gradeSlug)}`;
}
