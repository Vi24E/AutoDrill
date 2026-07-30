import type { WorksheetDto } from './drill-engine';

export type WorksheetMetadata = {
  /** Local generation date in YYYY-MM-DD form. */
  generated_date: string;
  /** The exact seed passed to the worksheet generator. */
  seed: string;
};

export type WorksheetDateGenerator = () => Date;

function twoDigits(value: number): string {
  return String(value).padStart(2, '0');
}

export function formatLocalGenerationDate(date: Date): string {
  return `${date.getFullYear()}-${twoDigits(date.getMonth() + 1)}-${twoDigits(date.getDate())}`;
}

export function createWorksheetMetadata(seed: string, date: Date): WorksheetMetadata {
  return { generated_date: formatLocalGenerationDate(date), seed };
}

export function formatWorksheetFooter(metadata: WorksheetMetadata): string {
  return `date: ${metadata.generated_date} / seed: ${metadata.seed}`;
}

/** Keep the Rust DTO and UI-only generation metadata as separate layers. */
export type WorksheetDocument = {
  worksheet: WorksheetDto;
  metadata: WorksheetMetadata;
};
