# Web / PDF boundary (alpha 1.1)

The web app is a Next.js 14 client under `apps/web`. q1 supports a default
recommended `genre → theme` selection and a `grade → genre → theme` selection
for grades 1 through 9. Implemented themes are declared once in
`src/domain/theme-registry.ts`; `curriculum.ts`, routes, sitemap, worksheet copy,
layout expectations, prompt kind, answer schema kind, and input capabilities are
derived from that registry. Recommended may regroup the same canonical themes
without duplicating them. Unimplemented Dummy themes disable generation and
printing and never receive a route or sitemap entry. Difficulty 1 through 5 is
sent unchanged to Rust.

The interactive client has
three observable states: settings (q1), the answer worksheet (q2), and a
separate browser PDF tab (q3). q1 generation enters q2; q1 print generates and
opens q3; q2 print uses the same `openWorksheetPdf` pipeline; and TOPに戻る
returns to q1. q2 enters with no selected problem and no keypad/control panel;
clicking a problem row reveals the panel, and Enter advances selection while
keeping the panel visible for the next problem.

The only seed input is on q1. An empty value is resolved at button-click time
through the injectable `seedGenerator` (Web Crypto `getRandomValues` first, a
same-alphabet/counter fallback otherwise); automatic seeds are exactly four
characters from `1-9`, `a-z`, and `A-Z` with `I`, `l`, and `O` excluded. A valid
non-empty value is one to sixteen characters from that alphabet and is passed
unchanged. q1 generation and q1 print each resolve a new blank-field seed,
while q2 print reuses the worksheet already in state. The exact resolved seed
and the local `YYYY-MM-DD` generation date are held in `WorksheetMetadata`; q2
has no seed input and displays this metadata in its paper footer. The Rust
identity boundary rejects invalid characters and 17+ characters with a typed
invalid request. Detailed q1 validation presentation remains a future UI
refinement.

## WASM adapter

`src/domain/wasm-adapter.ts` is the only production boundary for math and editor
operations. It calls the generated runtime methods `generate_worksheet`,
`apply_editor_action`, and `grade_answer` with schema-v3 JSON DTO strings. The
single-problem `generate_problem` export is also part of the Rust/WASM package
for direct callers; it is not substituted for worksheet generation. The adapter unwraps
`{schema_version, ok, data, error}` and
maps `generation_timeout` and `generation_attempt_limit` to distinct errors.
Worksheet grading is only sequencing: one `grade_answer` request per problem
with `{expected: problem.canonical_answer, actual: editorState.answer, answer_schema: problem.answer_schema}`.
The schema carries representation policy such as the reduced-fraction requirement; Web does not
reimplement generation, normalization, effort, or correctness rules.

All schema-v3 mathematical `i64`/`u64` payloads that may exceed JavaScript's
safe integer range cross JSON as canonical decimal strings. This includes
AnswerNode integer/coefficient values, integer answer-schema bounds, and
BigNum magnitudes. The adapter validates format and range without converting
them to JavaScript `number`; only scalar effort/vector quantities use numbers.

Structured editing also remains behind this boundary. Each Problem supplies an orthogonal typed
`input_interface`; the selected interface is included in every `apply_editor_action` request. A
`simple_numeric` one-digit addition problem renders only the conventional 10-digit keypad, while a
`structured_math` problem renders only its `allowed_structures` plus the shared numeric and editing controls.
`nan_error` preserves malformed raw text through DTO validation, display, editing, and grade projection without
coercing it to a number. `EditorState` carries the
display `answer` tree, an `active_path` to the selected numeric slot, and the
slot-local `cursor`. The adapter validates the complete answer tree against the selected interface on both
editor input/output and grading projections. Non-Clear editor calls require a valid current path/cursor and
SelectSlot requires an explicit valid target path/cursor; values are never silently repaired. Clear remains the
unconditional recovery action. Template and slot-selection actions are serialized to Rust;
the Web client only renders the returned fraction, mixed-fraction, decimal,
root, negative, plus-minus, and tuple nodes.

The runtime is intentionally injected as `window.__AUTODRILL_WASM__`. The
`src/wasm/load-generated.ts` seam loads the ignored package emitted under
`public/wasm/pkg` by `scripts/build-wasm.sh`; `AutoDrillApp` attaches its exports
to the global on the client when no engine is injected. Tests use an explicit fake
engine fixture and do not become a production fallback.

`scripts/build-wasm.sh` performs a target/tooling preflight and invokes
`wasm-pack --mode no-install`; it never installs a Rust target or a CLI. The
受入環境 uses Rust/Cargo 1.97.1 with `wasm32-unknown-unknown`, `wasm-pack
0.13.1`, and `wasm-bindgen 0.2.126`; the generated package loaded in the
browser after the wasm32 clock switched to `performance.now()`. `BrowserClock`
latched any host-clock throw, non-finite sample, backward sample, or first-read
failure and the generation boundary returned the typed `generation_timeout`
envelope for those failures; native callers continue to use `SystemClock`.

## Shared layout and PDF

`src/domain/layout.ts` owns A4 geometry and consumes the theme registry's
`columns × rows` layout. One-digit addition is 2×10; both linear-equation themes
are 2×8. The q2 paper converts the same top-origin cell positions to page-relative
CSS percentages, while the PDF renderer converts them to pdf-lib's bottom-origin
coordinates. Thus order, margins, row heights, and the central divider have one
serializable source of truth. `src/domain/problem-format.ts` is the shared typed
prompt→expression formatter used by Web and PDF, so equation text is not independently
reimplemented. For linear equations the expression is placed at the upper-left of
each cell and `x = [ ]` at the lower-right; the selected answer is scrolled above
the fixed keyboard before editing. Page one has empty answer boxes; page two contains
answers and has a 180° page rotation. Bytes are
generated client-side with `pdf-lib`, converted to a Blob, and navigated to via
a Blob URL. The q1 path opens a blank tab synchronously before the asynchronous
WASM generation so normal popup blocking does not discard the result.

The same metadata is passed to both PDF pages. The problem page draws its small
ASCII `date: ... / seed: ...` footer at the unrotated bottom-right. Because the
answer page is rotated 180°, its footer is drawn at the unrotated top-left;
after rotation it is physically bottom-right and remains readable. The pure
`getFooterPosition`/`getFooterPhysicalBounds` helpers make this transform
explicit and are covered by PDF tests.

Editor actions use a FIFO queue in the q2 client. The grading path awaits that
queue and reads the latest answer ref before sending the request, so a delayed
WASM editor action cannot be omitted by an immediately-following grade click.

## Dependencies and licensing

- Next.js, React, Vitest, and Testing Library: MIT. TypeScript: Apache-2.0.
  Their upstream license metadata is retained by the package manager.
- `pdf-lib`: MIT; used entirely in the browser, with no server or external
  service.
- `@fontsource/noto-sans-jp`: SIL Open Font License 1.1; bundled locally for
  the web UI. The PDF uses the PDF standard Helvetica font, so PDF generation
  has no runtime font fetch or network dependency.

The `pnpm-workspace.yaml` allow-list covers only local native build helpers
(`esbuild`, `unrs-resolver`) needed by the development toolchain.
