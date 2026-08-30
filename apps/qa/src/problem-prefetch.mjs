import { randomUUID } from 'node:crypto';
import { QaValidationError } from './constants.mjs';

export class ProblemPrefetchStore {
  constructor(runtime, { maxEntries = 8, prepareGenerated = (generated) => generated } = {}) {
    this.runtime = runtime;
    this.maxEntries = maxEntries;
    this.prepareGenerated = prepareGenerated;
    this.entries = new Map();
  }

  async reserve(skillId, { samplingMode = 'random', observations = [] } = {}) {
    const generated = this.prepareGenerated(await this.runtime.generateProblem({ skillId, samplingMode, observations }));
    const id = randomUUID();
    this.entries.set(id, { id, skillId, samplingMode, generated, consumed: false });
    while (this.entries.size > this.maxEntries) this.entries.delete(this.entries.keys().next().value);
    return { id, skill_id: skillId, sampling_mode: samplingMode };
  }

  renderPayload(id) {
    const entry = this.entries.get(id);
    if (!entry) throw new QaValidationError('Prefetched problem is no longer available.', 404);
    const payload = entry.generated.item.original_source_payload;
    return { worksheet: payload.worksheet, problem_index: payload.problem_index };
  }

  consume(id, skillId, samplingMode = 'random') {
    const entry = this.entries.get(id);
    if (!entry) throw new QaValidationError('Prefetched problem is no longer available.', 409);
    if (entry.consumed) throw new QaValidationError('Prefetched problem has already been used.', 409);
    if (entry.skillId !== skillId) throw new QaValidationError('Prefetched problem does not match the selected unit.', 409);
    if (entry.samplingMode !== samplingMode) throw new QaValidationError('Prefetched problem does not match the selected sampling mode.', 409);
    entry.consumed = true;
    return entry.generated;
  }
}
