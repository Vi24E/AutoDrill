// GENERATED FILE. DO NOT EDIT BY HAND.
// Source: drill-core::web_contract(). Run `pnpm contract:generate` after changing the Rust contract.

export const DRILL_CORE_CONTRACT = {
  "schema_version": 3,
  "themes": {
    "1": {
      "numeric_theme_id": 1,
      "generator_revision": 3,
      "skill_id": "jp.grade1.addition.one_digit",
      "curriculum_path": [
        "root",
        "小学1年生",
        "一桁の足し算"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "2": {
      "numeric_theme_id": 2,
      "generator_revision": 6,
      "skill_id": "jp.grade7.equation.linear.1",
      "curriculum_path": [
        "root",
        "中学1年生",
        "一次方程式",
        "一次方程式(1)"
      ],
      "layout": {
        "problem_count": 16,
        "columns": 2,
        "rows": 8
      }
    },
    "3": {
      "numeric_theme_id": 3,
      "generator_revision": 6,
      "skill_id": "jp.grade7.equation.linear.2",
      "curriculum_path": [
        "root",
        "中学1年生",
        "一次方程式",
        "一次方程式(2)"
      ],
      "layout": {
        "problem_count": 16,
        "columns": 2,
        "rows": 8
      }
    }
  },
  "grade_warning_codes": [
    "fraction_not_reduced",
    "redundant_negative",
    "redundant_decimal",
    "fraction_form_required",
    "integer_form_required"
  ]
} as const;

export type DrillCoreGradeWarningCode = typeof DRILL_CORE_CONTRACT.grade_warning_codes[number];
