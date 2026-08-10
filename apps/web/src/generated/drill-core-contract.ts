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
    },
    "4": {
      "numeric_theme_id": 4,
      "generator_revision": 1,
      "skill_id": "jp.grade1.subtraction.one_digit",
      "curriculum_path": [
        "root",
        "小学1年生",
        "一桁の引き算"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "5": {
      "numeric_theme_id": 5,
      "generator_revision": 1,
      "skill_id": "jp.grade2.addition.two_digit",
      "curriculum_path": [
        "root",
        "小学2年生",
        "二桁の足し算"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "6": {
      "numeric_theme_id": 6,
      "generator_revision": 1,
      "skill_id": "jp.grade2.multiplication.table",
      "curriculum_path": [
        "root",
        "小学2年生",
        "九九"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "7": {
      "numeric_theme_id": 7,
      "generator_revision": 1,
      "skill_id": "jp.grade7.signed.arithmetic.1",
      "curriculum_path": [
        "root",
        "中学1年生",
        "負の数の計算(1)"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "8": {
      "numeric_theme_id": 8,
      "generator_revision": 1,
      "skill_id": "jp.grade7.signed.arithmetic.2",
      "curriculum_path": [
        "root",
        "中学1年生",
        "負の数の計算(2)"
      ],
      "layout": {
        "problem_count": 20,
        "columns": 2,
        "rows": 10
      }
    },
    "9": {
      "numeric_theme_id": 9,
      "generator_revision": 1,
      "skill_id": "jp.grade5.fraction.addition",
      "curriculum_path": [
        "root",
        "小学5年生",
        "分数の足し算"
      ],
      "layout": {
        "problem_count": 16,
        "columns": 2,
        "rows": 8
      }
    },
    "10": {
      "numeric_theme_id": 10,
      "generator_revision": 1,
      "skill_id": "jp.grade6.fraction.multiplication",
      "curriculum_path": [
        "root",
        "小学6年生",
        "分数の掛け算"
      ],
      "layout": {
        "problem_count": 16,
        "columns": 2,
        "rows": 8
      }
    },
    "11": {
      "numeric_theme_id": 11,
      "generator_revision": 1,
      "skill_id": "jp.grade5.fraction.subtraction",
      "curriculum_path": [
        "root",
        "小学5年生",
        "分数の引き算"
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
