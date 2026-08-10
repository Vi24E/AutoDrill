import type { ArithmeticExpression, ArithmeticOperator, ProblemDto, RationalCoefficient } from './drill-engine';

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
function appendMinus(tokens: ProblemMathToken[]): void { tokens.push({ kind: 'minus' }); }
function appendRational(tokens: ProblemMathToken[], value: RationalCoefficient, absolute = false): void {
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
function needsParentheses(expression: ArithmeticExpression, parent: ArithmeticOperator, rightChild: boolean): boolean {
  if (expression.kind !== 'binary') return false;
  const child = arithmeticPrecedence(expression);
  const parentPrecedence = operatorPrecedence(parent);
  if (child < parentPrecedence) return true;
  return rightChild && child === parentPrecedence && (parent === 'subtract' || parent === 'divide');
}
function appendArithmeticExpression(tokens: ProblemMathToken[], expression: ArithmeticExpression, parent?: ArithmeticOperator, rightChild = false): void {
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
    appendText(tokens, operatorText(expression.operator));
    appendArithmeticExpression(tokens, expression.right, expression.operator, true);
  }
  if (parens) appendText(tokens, ')');
}

function appendCoefficientTerm(tokens: ProblemMathToken[], value: RationalCoefficient): boolean {
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
function appendLinearSide(tokens: ProblemMathToken[], coefficient: RationalCoefficient, constant: RationalCoefficient, negativeAsSubtraction: boolean): void {
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

export function problemExpressionTokens(problem: ProblemDto): readonly ProblemMathToken[] {
  if (problem.prompt.kind === 'addition') return [{ kind: 'text', text: `${problem.prompt.left} + ${problem.prompt.right} =` }];
  if (problem.prompt.kind === 'arithmetic') {
    const tokens: ProblemMathToken[] = [];
    appendArithmeticExpression(tokens, problem.prompt.expression);
    appendText(tokens, ' =');
    return tokens;
  }
  const tokens: ProblemMathToken[] = [];
  appendLinearSide(tokens, problem.prompt.a, problem.prompt.b, problem.prompt.left_negative_constant_as_subtraction);
  appendText(tokens, ' = ');
  appendLinearSide(tokens, problem.prompt.c, problem.prompt.d, problem.prompt.right_negative_constant_as_subtraction);
  return tokens;
}

export function problemExpression(problem: ProblemDto): string {
  return problemExpressionTokens(problem).map((token) => token.kind === 'text' ? token.text : token.kind === 'minus' ? '−' : `${token.numerator}/${token.denominator}`).join('');
}
