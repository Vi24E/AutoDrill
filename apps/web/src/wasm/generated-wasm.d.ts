declare module '/wasm/pkg/drill_wasm.js' {
  const init: (input?: unknown) => Promise<unknown>;
  export default init;
  export const generate_worksheet: (input: string) => string;
  export const generate_problem_set: (input: string) => string;
  export const parse_mathlive_answer: (input: string) => string;
  export const grade_answer: (input: string) => string;
}
