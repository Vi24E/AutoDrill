import { Fragment } from 'react';

export type RubyPart = string | readonly [base: string, reading: string];

export type RubyTextProps = {
  parts: readonly RubyPart[];
};

/** Render explicit, reviewable readings without changing the accessible label owner. */
export function RubyText({ parts }: RubyTextProps) {
  return parts.map((part, index) => (
    typeof part === 'string'
      ? <Fragment key={`${part}-${index}`}>{part}</Fragment>
      : (
        <ruby key={`${part[0]}-${index}`}>
          {part[0]}
          <rt aria-hidden="true">{part[1]}</rt>
        </ruby>
      )
  ));
}
