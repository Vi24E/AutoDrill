import type { AnswerInputStructure, AnswerNode, ArithmeticExpression, ArithmeticOperator, ProblemDto, RationalCoefficient } from './drill-engine';
import { problemExpressionTokens } from './problem-format';

export function answerNodeLatex(answer: AnswerNode): string {
  switch (answer.type) {
    case 'empty': return '';
    case 'integer': return answer.value.startsWith('-') ? `-${answer.value.slice(1)}` : answer.value;
    case 'exact_decimal': {
      const { coefficient, scale } = answer.value;
      const negative = coefficient.startsWith('-');
      const digits = negative ? coefficient.slice(1) : coefficient;
      const sign = negative ? '-' : '';
      if (scale === 0) return `${sign}${digits}.`;
      if (digits.length <= scale) return `${sign}0.${'0'.repeat(scale - digits.length)}${digits}`;
      const split = digits.length - scale;
      return `${sign}${digits.slice(0, split)}.${digits.slice(split)}`;
    }
    case 'nan_error': return answer.value;
    case 'fraction': return `\\frac{${answerNodeLatex(answer.value.numerator)}}{${answerNodeLatex(answer.value.denominator)}}`;
    case 'mixed_fraction': return `${answerNodeLatex(answer.value.whole)}\\frac{${answerNodeLatex(answer.value.numerator)}}{${answerNodeLatex(answer.value.denominator)}}`;
    case 'root': {
      const index = answer.value.index ? `[${answerNodeLatex(answer.value.index)}]` : '';
      return `\\sqrt${index}{${answerNodeLatex(answer.value.radicand)}}`;
    }
    case 'negative': return `-${answerNodeLatex(answer.value)}`;
    case 'plus_minus': return `\\pm${answerNodeLatex(answer.value)}`;
    case 'tuple': return answer.value.map(answerNodeLatex).join(',');
    case 'variable': return answer.value;
  }
}

function rationalLatex(value: RationalCoefficient): string {
  const sign = value.numerator < 0 ? '-' : '';
  const magnitude = Math.abs(value.numerator);
  return value.denominator === 1 ? `${sign}${magnitude}` : `${sign}\\frac{${magnitude}}{${value.denominator}}`;
}
function operatorPrecedence(operator: ArithmeticOperator): number { return operator === 'add' || operator === 'subtract' ? 1 : 2; }
function expressionPrecedence(expression: ArithmeticExpression): number { return expression.kind === 'binary' ? operatorPrecedence(expression.operator) : 3; }
function needsParentheses(expression: ArithmeticExpression, parent: ArithmeticOperator, rightChild: boolean): boolean {
  if (expression.kind !== 'binary') return false;
  const child = expressionPrecedence(expression);
  const p = operatorPrecedence(parent);
  return child < p || (rightChild && child === p && (parent === 'subtract' || parent === 'divide'));
}
function arithmeticExpressionLatex(expression: ArithmeticExpression, parent?: ArithmeticOperator, rightChild = false): string {
  let body: string;
  if (expression.kind === 'integer') body = expression.value < 0 ? `(${expression.value})` : String(expression.value);
  else if (expression.kind === 'rational') {
    const value = rationalLatex(expression.value);
    body = expression.value.numerator < 0 ? `(${value})` : value;
  } else {
    const operator = expression.operator === 'add' ? '+' : expression.operator === 'subtract' ? '-' : expression.operator === 'multiply' ? '\\times' : '\\div';
    body = `${arithmeticExpressionLatex(expression.left, expression.operator)}\\,${operator}\\,${arithmeticExpressionLatex(expression.right, expression.operator, true)}`;
  }
  return parent && needsParentheses(expression, parent, rightChild) ? `\\left(${body}\\right)` : body;
}

export function problemExpressionLatex(problem: ProblemDto): string {
  if (problem.prompt.kind === 'arithmetic') return `${arithmeticExpressionLatex(problem.prompt.expression)}\\,=`;
  return problemExpressionTokens(problem).map((token) => {
    if (token.kind === 'text') return token.text.replaceAll(' ', '\\,');
    if (token.kind === 'minus') return '-';
    return `\\frac{${token.numerator}}{${token.denominator}}`;
  }).join('');
}

export function mathTemplateLatex(structure: Exclude<AnswerInputStructure, 'decimal'>): string {
  switch (structure) {
    case 'fraction': return '\\frac{\\square}{\\square}';
    case 'mixed_fraction': return '\\square\\frac{\\square}{\\square}';
    case 'root': return '\\sqrt{\\square}';
    case 'negative': return '-\\square';
    case 'plus_minus': return '\\pm\\square';
    case 'tuple': return '\\square,\\square';
  }
}

export function mathTemplateInsertLatex(structure: Exclude<AnswerInputStructure, 'decimal'>): string {
  switch (structure) {
    case 'fraction': return '\\frac{\\placeholder{}}{\\placeholder{}}';
    case 'mixed_fraction': return '\\placeholder{}\\frac{\\placeholder{}}{\\placeholder{}}';
    case 'root': return '\\sqrt{\\placeholder{}}';
    case 'negative': return '-\\placeholder{}';
    case 'plus_minus': return '\\pm\\placeholder{}';
    case 'tuple': return '\\placeholder{},\\placeholder{}';
  }
}
