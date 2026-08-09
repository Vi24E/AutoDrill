import type { ProblemDto, RationalCoefficient } from './drill-engine';

export type ProblemMathToken =
  | { kind: 'text'; text: string }
  | { kind: 'minus' }
  | { kind: 'fraction'; numerator: number; denominator: number };

function appendText(tokens: ProblemMathToken[], text: string): void {
  if (text === '') return;
  const last = tokens[tokens.length - 1];
  if (last?.kind === 'text') last.text += text;
  else tokens.push({ kind: 'text', text });
}

function appendMinus(tokens: ProblemMathToken[]): void {
  tokens.push({ kind: 'minus' });
}

function appendRational(tokens: ProblemMathToken[], value: RationalCoefficient, absolute = false): void {
  const numerator = absolute ? Math.abs(value.numerator) : value.numerator;
  if (numerator < 0) appendMinus(tokens);
  if (value.denominator === 1) {
    appendText(tokens, String(Math.abs(numerator)));
    return;
  }
  tokens.push({ kind: 'fraction', numerator: Math.abs(numerator), denominator: value.denominator });
}

function appendCoefficientTerm(tokens: ProblemMathToken[], value: RationalCoefficient): boolean {
  if (value.numerator === 0) return false;
  const negative = value.numerator < 0;
  const magnitude = Math.abs(value.numerator);
  if (negative) appendMinus(tokens);
  if (value.denominator === 1) {
    if (magnitude !== 1) appendText(tokens, String(magnitude));
  } else {
    tokens.push({ kind: 'fraction', numerator: magnitude, denominator: value.denominator });
  }
  appendText(tokens, 'x');
  return true;
}

function appendLinearSide(
  tokens: ProblemMathToken[],
  coefficient: RationalCoefficient,
  constant: RationalCoefficient,
  negativeAsSubtraction: boolean,
): void {
  const hasX = appendCoefficientTerm(tokens, coefficient);
  if (!hasX) {
    if (constant.numerator === 0) appendText(tokens, '0');
    else appendRational(tokens, constant);
    return;
  }
  if (constant.numerator === 0) return;
  if (constant.numerator > 0) {
    appendText(tokens, ' + ');
    appendRational(tokens, constant);
    return;
  }
  if (negativeAsSubtraction) {
    appendText(tokens, ' ');
    appendMinus(tokens);
    appendText(tokens, ' ');
    appendRational(tokens, constant, true);
  } else {
    appendText(tokens, ' + (');
    appendRational(tokens, constant);
    appendText(tokens, ')');
  }
}

export function problemExpressionTokens(problem: ProblemDto): readonly ProblemMathToken[] {
  if (problem.prompt.kind === 'addition') {
    return [{ kind: 'text', text: `${problem.prompt.left} + ${problem.prompt.right} =` }];
  }
  const tokens: ProblemMathToken[] = [];
  appendLinearSide(
    tokens,
    problem.prompt.a,
    problem.prompt.b,
    problem.prompt.left_negative_constant_as_subtraction,
  );
  appendText(tokens, ' = ');
  appendLinearSide(
    tokens,
    problem.prompt.c,
    problem.prompt.d,
    problem.prompt.right_negative_constant_as_subtraction,
  );
  return tokens;
}

/** Plain-text projection for logs, test models and accessible labels. Visible math uses the token model. */
export function problemExpression(problem: ProblemDto): string {
  return problemExpressionTokens(problem).map((token) => (
    token.kind === 'text' ? token.text : token.kind === 'minus' ? '−' : `${token.numerator}/${token.denominator}`
  )).join('');
}
