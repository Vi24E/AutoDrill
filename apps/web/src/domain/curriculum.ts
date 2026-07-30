import {
  ADDITION_CURRICULUM_PATH,
  ADDITION_SKILL_ID,
  type CurriculumPathSegment,
} from '@/domain/drill-engine';

export type CurriculumUnit = {
  id: string;
  label: string;
  skillId: typeof ADDITION_SKILL_ID;
  curriculumPath: readonly CurriculumPathSegment[];
};

export type CurriculumArea = {
  id: string;
  label: string;
  units: readonly CurriculumUnit[];
};

export type CurriculumGrade = {
  id: string;
  label: string;
  areas: readonly CurriculumArea[];
};

const gradeSegment = ADDITION_CURRICULUM_PATH[1]!;
const unitSegment = ADDITION_CURRICULUM_PATH[2]!;

/**
 * q1 projects this hierarchy into the grade, area, and unit selects. Keeping
 * the labels and relationships here prevents each control from maintaining a
 * separate flat option list as the curriculum grows.
 */
export const CURRICULUM_TREE = [
  {
    id: gradeSegment.id,
    label: gradeSegment.label,
    areas: [
      {
        id: 'numbers-and-calculation',
        label: '数と計算',
        units: [
          {
            id: unitSegment.id,
            label: unitSegment.label,
            skillId: ADDITION_SKILL_ID,
            curriculumPath: ADDITION_CURRICULUM_PATH,
          },
        ],
      },
    ],
  },
] as const satisfies readonly CurriculumGrade[];

export type CurriculumSelection = {
  grade: CurriculumGrade;
  area: CurriculumArea;
  unit: CurriculumUnit;
};

export function findCurriculumSelection(skillId: string): CurriculumSelection {
  for (const grade of CURRICULUM_TREE) {
    for (const area of grade.areas) {
      const unit = area.units.find((candidate) => candidate.skillId === skillId);
      if (unit) return { grade, area, unit };
    }
  }

  const grade: CurriculumGrade = CURRICULUM_TREE[0];
  const area: CurriculumArea = grade.areas[0];
  const unit: CurriculumUnit = area.units[0];
  return { grade, area, unit };
}
