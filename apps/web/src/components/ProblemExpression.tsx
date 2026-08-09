import { problemExpression, problemExpressionTokens } from '@/domain/problem-format';
import type { ProblemDto } from '@/domain/drill-engine';

export function ProblemExpression({ problem }: { problem: ProblemDto }) {
  const tokens = problemExpressionTokens(problem);
  return (
    <math className="problem-math-expression" aria-label={problemExpression(problem)}>
      <mrow>
        {tokens.map((token, index) => token.kind === 'text' ? (
          <mtext key={index}>{token.text}</mtext>
        ) : token.kind === 'minus' ? (
          <mo key={index}>−</mo>
        ) : (
          <mfrac key={index}>
            <mn>{token.numerator}</mn>
            <mn>{token.denominator}</mn>
          </mfrac>
        ))}
      </mrow>
    </math>
  );
}
