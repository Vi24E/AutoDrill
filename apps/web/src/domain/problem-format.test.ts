import { describe, expect, it } from 'vitest';

import { liarStatementText, problemExpression, problemExpressionTokens } from './problem-format';
import { fixtureWorksheet, linearFixtureWorksheet } from '@/test/fixtures';
import { answerNodeLatex, answerPrefixLatex, problemExpressionLatex } from './mathlive-format';
import type { LinearExpression, ProblemDto, RationalCoefficient } from './drill-engine';

function linearProblem(left: LinearExpression, right: LinearExpression): ProblemDto {
  const base = linearFixtureWorksheet(2).problems[0]!;
  return { ...base, prompt: { kind: 'linear_equation', left, right } };
}

const variable = (variable: 'x' | 'y' = 'x'): LinearExpression => ({ kind: 'variable', variable });
const integer = (value: number): LinearExpression => ({ kind: 'constant', value: { kind: 'integer', value } });
const scale = (value: number, expression: LinearExpression): LinearExpression => ({
  kind: 'scale', factor: { kind: 'integer', value }, expression,
});

const add = (left: LinearExpression, right: LinearExpression): LinearExpression => ({ kind: 'add', left, right });
const subtract = (left: LinearExpression, right: LinearExpression): LinearExpression => ({ kind: 'subtract', left, right });

const q = (numerator: number, denominator = 1): RationalCoefficient => ({ numerator, denominator });

describe('problemExpression', () => {
  it('formats answer prefixes with exactly one MathLive spacing command', () => {
    expect(answerPrefixLatex('x =')).toBe(String.raw`x\,=`);
  });

  it('omits coefficient 1 while retaining coefficient -1 in linear-expression ASTs', () => {
    expect(problemExpression(linearProblem(variable(), scale(2, variable())))).toBe('x = 2x');
    expect(problemExpression(linearProblem(scale(-1, variable()), scale(2, variable())))).toBe('−x = 2x');
    expect(problemExpression(linearProblem(scale(3, variable()), integer(0)))).toBe('3x = 0');
  });

  it('preserves parenthesized, fractional, and decimal linear-equation surfaces', () => {
    const left: LinearExpression = {
      kind: 'scale',
      factor: { kind: 'fraction', value: q(1, 2) },
      expression: {
        kind: 'subtract',
        left: variable(),
        right: integer(3),
      },
    };
    const right: LinearExpression = {
      kind: 'scale',
      factor: { kind: 'exact_decimal', coefficient: 5, scale: 1 },
      expression: variable(),
    };
    const problem = linearProblem(left, right);
    expect(problemExpression(problem)).toBe('1/2(x − 3) = 0.5x');
    expect(problemExpressionTokens(problem)).toEqual([
      { kind: 'fraction', numerator: 1, denominator: 2 },
      { kind: 'text', text: '(x ' },
      { kind: 'minus' },
      { kind: 'text', text: ' 3) = 0.5x' },
    ]);
    expect(problemExpressionLatex(problem)).toContain('\\frac{1}{2}');
    expect(problemExpressionLatex(problem)).toContain('0.5x');
  });

  it('keeps exact decimal operands in decimal notation', () => {
    const base = fixtureWorksheet().problems[0]!;
    const problem: ProblemDto = {
      ...base,
      prompt: {
        kind: 'arithmetic',
        expression: {
          kind: 'binary', operator: 'subtract',
          left: { kind: 'exact_decimal', coefficient: 42, scale: 1 },
          right: { kind: 'exact_decimal', coefficient: 7, scale: 3 },
        },
      },
    };
    expect(problemExpression(problem)).toBe('4.2 − 0.007 =');
    expect(problemExpressionLatex(problem)).toBe('4.2\\,-\\,0.007\\,=');
  });

  it('formats quadratic equations and bounded quadratic-formula answers for MathLive', () => {
    const base = fixtureWorksheet().problems[0]!;
    const problem: ProblemDto = {
      ...base,
      prompt: {
        kind: 'quadratic_equation',
        equation: {
          left: {
            kind: 'add',
            left: {
              kind: 'subtract',
              left: { kind: 'scale', factor: { kind: 'fraction', value: q(1, 2) }, expression: { kind: 'square', expression: variable() } },
              right: { kind: 'linear', expression: scale(3, variable()) },
            },
            right: { kind: 'linear', expression: { kind: 'constant', value: { kind: 'fraction', value: q(1, 4) } } },
          },
          right: { kind: 'linear', expression: integer(0) },
        },
        solve_method: 'formula',
      },
      canonical_answer: {
        type: 'fraction',
        value: {
          numerator: {
            type: 'binary',
            value: {
              operator: 'add',
              left: { type: 'integer', value: '-3' },
              right: {
                type: 'plus_minus',
                value: {
                  type: 'binary',
                  value: {
                    operator: 'multiply',
                    left: { type: 'integer', value: '2' },
                    right: { type: 'root', value: { radicand: { type: 'integer', value: '5' }, index: null } },
                  },
                },
              },
            },
          },
          denominator: { type: 'integer', value: '4' },
        },
      },
    };
    expect(problemExpression(problem)).toBe('1/2x² − 3x + 1/4 = 0');
    const expressionLatex = problemExpressionLatex(problem);
    expect(expressionLatex).toContain('\\frac{1}{2}');
    expect(expressionLatex).toContain('x^2');
    expect(expressionLatex).not.toContain('²');
    const answerLatex = answerNodeLatex(problem.canonical_answer);
    expect(answerLatex).toContain('\\pm');
    expect(answerLatex).toContain('2\\sqrt{5}');
    expect(answerLatex).toContain('\\frac');
  });

  it('formats simultaneous equations as a two-row MathLive case', () => {
    const base = fixtureWorksheet().problems[0]!;
    const problem: ProblemDto = {
      ...base,
      prompt: { kind: 'simultaneous_equation', equations: [{ left: subtract(scale(2, variable()), variable('y')), right: integer(7) }, { left: add(scale(-1, variable()), scale(3, variable('y'))), right: integer(-4) }], solve_method: 'elimination' },
      answer_schema: { kind: 'ordered_pair' },
      canonical_answer: { type: 'tuple', value: [{ type: 'integer', value: '2' }, { type: 'integer', value: '-3' }] },
    };
    expect(problemExpression(problem)).toBe('2x − y = 7 / −x + 3y = −4');
    const latex = problemExpressionLatex(problem);
    expect(latex).toContain('\\begin{cases}');
    expect(latex).toContain('2x\\,-\\,y\\,=\\,7');
    expect(latex).toContain('-x\\,+\\,3y\\,=\\,-4');
    expect(latex).toContain('\\\\');
    expect(latex).toContain('\\end{cases}');
  });

  it('formats arithmetic ASTs with exact fractions and precedence for MathLive and PDF', () => {
    const base = fixtureWorksheet().problems[0]!;
    const problem: ProblemDto = {
      ...base,
      prompt: {
        kind: 'arithmetic',
        expression: {
          kind: 'binary',
          operator: 'multiply',
          left: { kind: 'rational', value: q(-1, 2) },
          right: {
            kind: 'binary',
            operator: 'add',
            left: { kind: 'integer', value: 3 },
            right: { kind: 'integer', value: -4 },
          },
        },
      },
    };
    expect(problemExpression(problem)).toBe('(−1/2) × (3 + (−4)) =');
    expect(problemExpressionLatex(problem)).toBe('(-\\frac{1}{2})\\,\\times\\,\\left(3\\,+\\,(-4)\\right)\\,=');
    expect(problemExpressionTokens(problem).some((token) => token.kind === 'fraction')).toBe(true);
  });

  it('renders improper values as mixed numbers only in the standard elementary fraction units', () => {
    const base = fixtureWorksheet().problems[0]!;
    const expression = {
      kind: 'binary' as const,
      operator: 'multiply' as const,
      left: { kind: 'rational' as const, value: q(7, 3) },
      right: { kind: 'rational' as const, value: q(3, 1) },
    };
    const standard: ProblemDto = {
      ...base,
      numeric_theme_id: 10,
      prompt: { kind: 'arithmetic', expression },
    };
    const summary: ProblemDto = {
      ...standard,
      numeric_theme_id: 23,
    };

    expect(problemExpression(standard)).toBe('21/3 × 3 =');
    expect(problemExpressionTokens(standard)).toEqual([
      { kind: 'text', text: '2' },
      { kind: 'fraction', numerator: 1, denominator: 3 },
      { kind: 'text', text: ' × 3 =' },
    ]);
    expect(problemExpressionLatex(standard)).toBe('2\\frac{1}{3}\\,\\times\\,3\\,=');
    expect(problemExpressionTokens(summary)).toEqual([
      { kind: 'fraction', numerator: 7, denominator: 3 },
      { kind: 'text', text: ' × 3 =' },
    ]);
    expect(problemExpressionLatex(summary)).toBe('\\frac{7}{3}\\,\\times\\,3\\,=');
  });

  it('formats every liar-puzzle statement form in Japanese', () => {
    expect(liarStatementText({ kind: 'says_liar', person: 2 })).toBe('Bさんはうそつきだ。');
    expect(liarStatementText({ kind: 'says_not_liar', person: 3 })).toBe('Cさんはうそつきではない。');
    expect(liarStatementText({ kind: 'exactly_one_liar', first: 1, second: 4 })).toBe('AさんとDさんのうち、うそつきは1人だけだ。');
    expect(liarStatementText({ kind: 'exact_liar_count', count: 2 })).toBe('このなかの2人がうそつきだ。');
    expect(liarStatementText({ kind: 'both_liar', first: 1, second: 3 })).toBe('AさんとCさんはうそつきだ。');
    expect(liarStatementText({ kind: 'both_not_liar', first: 2, second: 4 })).toBe('BさんとDさんはうそつきではない。');
  });

});
