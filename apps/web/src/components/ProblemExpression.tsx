import { MathLiveStatic } from '@/components/MathLiveMath';
import { problemExpression } from '@/domain/problem-format';
import { problemExpressionLatex } from '@/domain/mathlive-format';
import type { ProblemDto } from '@/domain/drill-engine';

export function ProblemExpression({ problem }: { problem: ProblemDto }) {
  return (
    <MathLiveStatic
      className="problem-math-expression"
      latex={problemExpressionLatex(problem)}
      ariaLabel={problemExpression(problem)}
    />
  );
}
