export type WorksheetGradeBand = 'early-elementary' | 'late-elementary' | 'junior-high';

/** Shared typography band for Web and print worksheets from canonical numeric grade metadata. */
export function worksheetGradeBand(grade: number): WorksheetGradeBand {
  if (grade >= 7) return 'junior-high';
  if (grade >= 4) return 'late-elementary';
  return 'early-elementary';
}

export function worksheetGradeBandClass(grade: number): string {
  return `worksheet-grade-${worksheetGradeBand(grade)}`;
}
