import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ArithmeticOperator, type LiarStatement, type LinearExpression, type LinearScalar, type ProblemDto, type RationalCoefficient } from './drill-engine';
import { findThemeDefinitionByNumericId } from './theme-registry';


export function liarPersonLabel(person: number): string {
  return String.fromCharCode('A'.charCodeAt(0) + person - 1);
}

export function liarStatementText(statement: LiarStatement): string {
  switch (statement.kind) {
    case 'says_liar': return `${liarPersonLabel(statement.person)}さんはうそつきだ。`;
    case 'says_not_liar': return `${liarPersonLabel(statement.person)}さんはうそつきではない。`;
    case 'exactly_one_liar': return `${liarPersonLabel(statement.first)}さんと${liarPersonLabel(statement.second)}さんのうち、うそつきは1人だけだ。`;
    case 'exact_liar_count': return `このなかの${statement.count}人がうそつきだ。`;
    case 'both_liar': return `${liarPersonLabel(statement.first)}さんと${liarPersonLabel(statement.second)}さんはうそつきだ。`;
    case 'both_not_liar': return `${liarPersonLabel(statement.first)}さんと${liarPersonLabel(statement.second)}さんはうそつきではない。`;
  }
}

/** Semantic tokens for accessible/plain-text projections. Visual math is rendered by MathLive. */
export type MathToken =
  | { kind: 'text'; text: string }
  | { kind: 'minus' }
  | { kind: 'fraction'; numerator: number; denominator: number };

function appendText(tokens: MathToken[], text: string): void {
  if (text === '') return;
  const last = tokens[tokens.length - 1];
  if (last?.kind === 'text') last.text += text;
  else tokens.push({ kind: 'text', text });
}
function appendMinus(tokens: MathToken[]): void { tokens.push({ kind: 'minus' }); }
function appendRational(tokens: MathToken[], value: RationalCoefficient, absolute = false, mixed = false): void {
  const numerator = absolute ? Math.abs(value.numerator) : value.numerator;
  const magnitude = Math.abs(numerator);
  if (numerator < 0) appendMinus(tokens);
  if (value.denominator === 1) { appendText(tokens, String(magnitude)); return; }
  if (mixed && magnitude > value.denominator) {
    appendText(tokens, String(Math.floor(magnitude / value.denominator)));
    const remainder = magnitude % value.denominator;
    if (remainder !== 0) tokens.push({ kind: 'fraction', numerator: remainder, denominator: value.denominator });
    return;
  }
  tokens.push({ kind: 'fraction', numerator: magnitude, denominator: value.denominator });
}

function operatorPrecedence(operator: ArithmeticOperator): number {
  return operator === 'add' || operator === 'subtract' ? 1 : 2;
}
function arithmeticPrecedence(expression: ArithmeticExpression): number {
  return expression.kind === 'binary' ? operatorPrecedence(expression.operator) : 3;
}
function operatorText(operator: ArithmeticOperator): string {
  switch (operator) {
    case 'add': return ' + ';
    case 'subtract': return ' − ';
    case 'multiply': return ' × ';
    case 'divide': return ' ÷ ';
  }
}
function appendOperator(tokens: MathToken[], operator: ArithmeticOperator): void {
  if (operator === 'subtract') {
    appendText(tokens, ' ');
    appendMinus(tokens);
    appendText(tokens, ' ');
  } else {
    appendText(tokens, operatorText(operator));
  }
}
function needsParentheses(expression: ArithmeticExpression, parent: ArithmeticOperator, rightChild: boolean): boolean {
  if (expression.kind !== 'binary') return false;
  const child = arithmeticPrecedence(expression);
  const parentPrecedence = operatorPrecedence(parent);
  if (child < parentPrecedence) return true;
  return rightChild && child === parentPrecedence && (parent === 'subtract' || parent === 'divide');
}
function exactDecimalExpressionText(coefficient: number, scale: number): string {
  const negative = coefficient < 0;
  const digits = String(Math.abs(coefficient));
  const padded = digits.padStart(scale + 1, '0');
  const split = padded.length - scale;
  return `${negative ? '−' : ''}${padded.slice(0, split)}.${padded.slice(split)}`;
}
function appendArithmeticExpression(tokens: MathToken[], expression: ArithmeticExpression, parent?: ArithmeticOperator, rightChild = false, mixedFractions = false): void {
  const parens = parent ? needsParentheses(expression, parent, rightChild) : false;
  if (parens) appendText(tokens, '(');
  if (expression.kind === 'integer') {
    if (expression.value < 0) {
      appendText(tokens, '('); appendMinus(tokens); appendText(tokens, `${Math.abs(expression.value)})`);
    } else appendText(tokens, String(expression.value));
  } else if (expression.kind === 'rational') {
    if (expression.value.numerator < 0) {
      appendText(tokens, '('); appendMinus(tokens); appendRational(tokens, expression.value, true); appendText(tokens, ')');
    } else appendRational(tokens, expression.value, false, mixedFractions);
  } else if (expression.kind === 'exact_decimal') {
    appendText(tokens, exactDecimalExpressionText(expression.coefficient, expression.scale));
  } else {
    appendArithmeticExpression(tokens, expression.left, expression.operator, false, mixedFractions);
    appendOperator(tokens, expression.operator);
    appendArithmeticExpression(tokens, expression.right, expression.operator, true, mixedFractions);
  }
  if (parens) appendText(tokens, ')');
}

export function arithmeticLeafText(expression: ArithmeticExpression): string {
  if (expression.kind === 'integer') return String(expression.value);
  if (expression.kind === 'exact_decimal') return exactDecimalExpressionText(expression.coefficient, expression.scale);
  const tokens: MathToken[] = [];
  appendArithmeticExpression(tokens, expression);
  return mathTokensText(tokens);
}

function appendLinearScalar(tokens: MathToken[], value: LinearScalar, omitPositiveOne = false): void {
  if (value.kind === 'integer') {
    if (value.value < 0) appendMinus(tokens);
    const magnitude = Math.abs(value.value);
    if (!(omitPositiveOne && magnitude === 1)) appendText(tokens, String(magnitude));
    return;
  }
  if (value.kind === 'fraction') {
    const negative = value.value.numerator < 0;
    if (negative) appendMinus(tokens);
    const magnitude = { ...value.value, numerator: Math.abs(value.value.numerator) };
    if (!(omitPositiveOne && magnitude.numerator === magnitude.denominator)) appendRational(tokens, magnitude);
    return;
  }
  const negative = value.coefficient < 0;
  if (negative) appendMinus(tokens);
  const magnitude = Math.abs(value.coefficient);
  if (!(omitPositiveOne && magnitude === 10 ** value.scale)) {
    appendText(tokens, exactDecimalExpressionText(magnitude, value.scale));
  }
}

function linearExpressionNeedsParentheses(expression: LinearExpression): boolean {
  return expression.kind === 'add' || expression.kind === 'subtract';
}

function appendLinearExpression(tokens: MathToken[], expression: LinearExpression): void {
  if (expression.kind === 'variable') {
    appendText(tokens, expression.variable);
    return;
  }
  if (expression.kind === 'constant') {
    appendLinearScalar(tokens, expression.value);
    return;
  }
  if (expression.kind === 'add' || expression.kind === 'subtract') {
    appendLinearExpression(tokens, expression.left);
    if (expression.kind === 'add') appendText(tokens, ' + ');
    else { appendText(tokens, ' '); appendMinus(tokens); appendText(tokens, ' '); }
    appendLinearExpression(tokens, expression.right);
    return;
  }
  appendLinearScalar(tokens, expression.factor, true);
  const parens = linearExpressionNeedsParentheses(expression.expression);
  if (parens) appendText(tokens, '(');
  appendLinearExpression(tokens, expression.expression);
  if (parens) appendText(tokens, ')');
}

function appendPolynomialTerm(tokens: MathToken[], coefficient: RationalCoefficient, variable: 'x²' | 'x', first: boolean): boolean {
  if (coefficient.numerator === 0) return false;
  const negative = coefficient.numerator < 0;
  if (!first) {
    appendText(tokens, negative ? ' ' : ' + ');
    if (negative) { appendMinus(tokens); appendText(tokens, ' '); }
  } else if (negative) {
    appendMinus(tokens);
  }
  const magnitude = { ...coefficient, numerator: Math.abs(coefficient.numerator) };
  if (!(magnitude.numerator === magnitude.denominator)) appendRational(tokens, magnitude);
  appendText(tokens, variable);
  return true;
}

function appendPolynomialConstant(tokens: MathToken[], value: RationalCoefficient, first: boolean): boolean {
  if (value.numerator === 0) return false;
  if (!first) {
    if (value.numerator < 0) { appendText(tokens, ' '); appendMinus(tokens); appendText(tokens, ' '); appendRational(tokens, { ...value, numerator: Math.abs(value.numerator) }); }
    else { appendText(tokens, ' + '); appendRational(tokens, value); }
  } else appendRational(tokens, value);
  return true;
}


function appendLinearEquationSurface(tokens: MathToken[], equation: { left: LinearExpression; right: LinearExpression }): void {
  appendLinearExpression(tokens, equation.left);
  appendText(tokens, ' = ');
  appendLinearExpression(tokens, equation.right);
}


function quadraticExpressionTokens(problem: ProblemDto): readonly MathToken[] {
  if (problem.prompt.kind !== 'quadratic_equation') return [];
  const { form, a, b, c } = problem.prompt;
  const tokens: MathToken[] = [];
  if (form === 'factored_scale') {
    if (!(a.numerator === a.denominator)) appendRational(tokens, a);
    appendText(tokens, '(');
    appendText(tokens, 'x²');
    appendPolynomialTerm(tokens, b, 'x', false);
    appendPolynomialConstant(tokens, c, false);
    appendText(tokens, ') = 0');
    return tokens;
  }
  appendPolynomialTerm(tokens, a, 'x²', true);
  if (form === 'square_equals_constant') {
    appendText(tokens, ' = ');
    appendRational(tokens, c);
    return tokens;
  }
  if (form === 'standard') appendPolynomialTerm(tokens, b, 'x', false);
  appendPolynomialConstant(tokens, c, false);
  appendText(tokens, ' = 0');
  return tokens;
}

export function usesMixedFractionPresentation(problem: ProblemDto): boolean {
  return findThemeDefinitionByNumericId(problem.numeric_theme_id)?.presentation.fraction
    === 'mixed_number_when_improper';
}

export function problemExpressionTokens(problem: ProblemDto, includeAnswerEquals = true): readonly MathToken[] {
  if (problem.prompt.kind === 'addition') return [{ kind: 'text', text: `${problem.prompt.left} + ${problem.prompt.right}${includeAnswerEquals ? ' =' : ''}` }];
  if (problem.prompt.kind === 'arithmetic') {
    const tokens: MathToken[] = [];
    appendArithmeticExpression(tokens, problem.prompt.expression, undefined, false, usesMixedFractionPresentation(problem));
    if (includeAnswerEquals) appendText(tokens, ' =');
    return tokens;
  }
  if (problem.prompt.kind === 'column_arithmetic') {
    const operator = problem.prompt.operator === 'add' ? '+'
      : problem.prompt.operator === 'subtract' ? '−'
        : problem.prompt.operator === 'multiply' ? '×' : '÷';
    return [{ kind: 'text', text: `${arithmeticLeafText(problem.prompt.left)} ${operator} ${arithmeticLeafText(problem.prompt.right)}${includeAnswerEquals ? ' =' : ''}` }];
  }
  if (problem.prompt.kind === 'quadratic_equation') return quadraticExpressionTokens(problem);
  if (problem.prompt.kind === 'simultaneous_equation') {
    const tokens: MathToken[] = [];
    appendLinearEquationSurface(tokens, problem.prompt.equations[0]);
    appendText(tokens, ' / ');
    appendLinearEquationSurface(tokens, problem.prompt.equations[1]);
    return tokens;
  }
  if (problem.prompt.kind === 'liar_puzzle') {
    return [{ kind: 'text', text: problem.prompt.statements.map((statement, index) => `${liarPersonLabel(index + 1)}さん「${liarStatementText(statement)}」`).join(' / ') }];
  }
  if (problem.prompt.kind === 'mini_sudoku') {
    return [{ kind: 'text', text: '4×4 数独' }];
  }
  const tokens: MathToken[] = [];
  appendLinearExpression(tokens, problem.prompt.left);
  appendText(tokens, ' = ');
  appendLinearExpression(tokens, problem.prompt.right);
  return tokens;
}

/** Canonical-answer projection for printed answers. Current generators use integers and simple rationals. */
export function answerNodeMathTokens(answer: AnswerNode): readonly MathToken[] {
  switch (answer.type) {
    case 'integer': {
      const value = BigInt(answer.value);
      return value < 0n
        ? [{ kind: 'minus' }, { kind: 'text', text: (-value).toString() }]
        : [{ kind: 'text', text: value.toString() }];
    }
    case 'fraction': {
      const numerator = answer.value.numerator;
      const denominator = answer.value.denominator;
      if (numerator.type === 'integer' && denominator.type === 'integer') {
        const top = BigInt(numerator.value);
        const bottom = BigInt(denominator.value);
        const sign = (top < 0n) !== (bottom < 0n);
        const absTop = top < 0n ? -top : top;
        const absBottom = bottom < 0n ? -bottom : bottom;
        if (absTop <= BigInt(Number.MAX_SAFE_INTEGER) && absBottom <= BigInt(Number.MAX_SAFE_INTEGER)) {
          return [
            ...(sign ? [{ kind: 'minus' } as const] : []),
            { kind: 'fraction', numerator: Number(absTop), denominator: Number(absBottom) },
          ];
        }
      }
      break;
    }
    case 'negative': {
      return [{ kind: 'minus' }, ...answerNodeMathTokens(answer.value)];
    }
    default:
      break;
  }
  return [{ kind: 'text', text: answerNodeText(answer).replaceAll('−', '-') }];
}

export function mathTokensText(tokens: readonly MathToken[]): string {
  return tokens.map((token) => token.kind === 'text' ? token.text : token.kind === 'minus' ? '−' : `${token.numerator}/${token.denominator}`).join('');
}

export function problemExpression(problem: ProblemDto, includeAnswerEquals = true): string {
  return mathTokensText(problemExpressionTokens(problem, includeAnswerEquals));
}
