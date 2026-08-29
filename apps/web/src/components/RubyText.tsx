import { Fragment } from 'react';

export type RubyPart = string | readonly [base: string, reading: string];

export type RubyTextProps = {
  parts: readonly RubyPart[];
};

/** Render explicit, reviewable readings as one inline text run. */
export function RubyText({ parts }: RubyTextProps) {
  return (
    <span className="ruby-text">
      {parts.map((part, index) => (
        typeof part === 'string'
          ? <Fragment key={`${part}-${index}`}>{part}</Fragment>
          : (
            <ruby key={`${part[0]}-${index}`}>
              {part[0]}
              <rt aria-hidden="true">{part[1]}</rt>
            </ruby>
          )
      ))}
    </span>
  );
}
