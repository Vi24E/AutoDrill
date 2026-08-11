import { answerNodeText, type AnswerNode, type ArithmeticExpression, type ArithmeticOperator, type ProblemDto, type RationalCoefficient } from './drill-engine';

/** Semantic tokens for accessible/plain-text projections. Visual math is rendered by MathLive. */
export type MathToken =
  | { kind: 'text'; text: string }
  | { kind: 'minus' }
  | { kind: 'fraction'; numerator: number; denominator: number };

/** Backward-compatible name retained for existing callers. */
export type ProblemMathToken = MathToken;

function appendText(tokens: MathToken[], text: string): void {
  if (text === '') return;
  const last = tokens[tokens.length - 1];
  if (last?.kind === 'text') last.text += text;
  else tokens.push({ kind: 'text', text });
}
function appendMinus(tokens: MathToken[]): void { tokens.push({ kind: 'minus' }); }
function appendRational(tokens: MathToken[], value: RationalCoefficient, absolute = false): void {
  const numerator = absolute ? Math.abs(value.numerator) : value.numerator;
  if (numerator < 0) appendMinus(tokens);
  if (value.denominator === 1) appendText(tokens, String(Math.abs(numerator)));
  else tokens.push({ kind: 'fraction', numerator: Math.abs(numerator), denominator: value.denominator });
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
function appendArithmeticExpression(tokens: MathToken[], expression: ArithmeticExpression, parent?: ArithmeticOperator, rightChild = false): void {
  const parens = parent ? needsParentheses(expression, parent, rightChild) : false;
  if (parens) appendText(tokens, '(');
  if (expression.kind === 'integer') {
    if (expression.value < 0) {
      appendText(tokens, '('); appendMinus(tokens); appendText(tokens, `${Math.abs(expression.value)})`);
    } else appendText(tokens, String(expression.value));
  } else if (expression.kind === 'rational') {
    if (expression.value.numerator < 0) {
      appendText(tokens, '('); appendMinus(tokens); appendRational(tokens, expression.value, true); appendText(tokens, ')');
    } else appendRational(tokens, expression.value);
  } else {
    appendArithmeticExpression(tokens, expression.left, expression.operator, false);
    appendOperator(tokens, expression.operator);
    appendArithmeticExpression(tokens, expression.right, expression.operator, true);
  }
  if (parens) appendText(tokens, ')');
}

function appendCoefficientTerm(tokens: MathToken[], value: RationalCoefficient): boolean {
  if (value.numerator === 0) return false;
  const negative = value.numerator < 0;
  const magnitude = Math.abs(value.numerator);
  if (negative) appendMinus(tokens);
  if (value.denominator === 1) {
    if (magnitude !== 1) appendText(tokens, String(magnitude));
  } else tokens.push({ kind: 'fraction', numerator: magnitude, denominator: value.denominator });
  appendText(tokens, 'x');
  return true;
}
function appendLinearSide(tokens: MathToken[], coefficient: RationalCoefficient, constant: RationalCoefficient, negativeAsSubtraction: boolean): void {
  const hasX = appendCoefficientTerm(tokens, coefficient);
  if (!hasX) {
    if (constant.numerator === 0) appendText(tokens, '0'); else appendRational(tokens, constant);
    return;
  }
  if (constant.numerator === 0) return;
  if (constant.numerator > 0) { appendText(tokens, ' + '); appendRational(tokens, constant); return; }
  if (negativeAsSubtraction) { appendText(tokens, ' '); appendMinus(tokens); appendText(tokens, ' '); appendRational(tokens, constant, true); }
  else { appendText(tokens, ' + ('); appendRational(tokens, constant); appendText(tokens, ')'); }
}

export function problemExpressionTokens(problem: ProblemDto): readonly MathToken[] {
  if (problem.prompt.kind === 'addition') return [{ kind: 'text', text: `${problem.prompt.left} + ${problem.prompt.right} =` }];
  if (problem.prompt.kind === 'arithmetic') {
    const tokens: MathToken[] = [];
    appendArithmeticExpression(tokens, problem.prompt.expression);
    appendText(tokens, ' =');
    return tokens;
  }
  const tokens: MathToken[] = [];
  appendLinearSide(tokens, problem.prompt.a, problem.prompt.b, problem.prompt.left_negative_constant_as_subtraction);
  appendText(tokens, ' = ');
  appendLinearSide(tokens, problem.prompt.c, problem.prompt.d, problem.prompt.right_negative_constant_as_subtraction);
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

export function problemExpression(problem: ProblemDto): string {
  return mathTokensText(problemExpressionTokens(problem));
}
