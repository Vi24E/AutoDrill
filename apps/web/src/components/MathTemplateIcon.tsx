import { MathLiveStatic } from '@/components/MathLiveMath';
import { mathTemplateLatex } from '@/domain/mathlive-format';
import type { AnswerInputStructure } from '@/domain/drill-engine';

type MathTemplateIconProps = {
  structure: Exclude<AnswerInputStructure, 'decimal' | 'arithmetic'>;
};

export function MathTemplateIcon({ structure }: MathTemplateIconProps) {
  return (
    <MathLiveStatic
      className="math-template-icon"
      latex={mathTemplateLatex(structure)}
      ariaLabel={structure}
    />
  );
}
