export type CorpusState = 'Published' | 'Deferred';

export interface CorpusRecord {
  experiment: string;
  finding: string;
  state: CorpusState;
  home: string;
  reason?: string;
}

export const corpus: CorpusRecord[] = [
  {
    experiment: 'EXP-01',
    finding: 'Natural accountable leadership can improve a coherent artifact when a real, differentiated seam exists. It is a quality result, not a speed rule.',
    state: 'Published',
    home: '/journal/one-worker-is-a-default-not-a-rule/'
  },
  {
    experiment: 'EXP-02',
    finding: 'Most proposed collaboration affordances did not repay their cost. A sparse factual responsibility floor remained useful.',
    state: 'Published',
    home: '/journal/one-worker-is-a-default-not-a-rule/'
  },
  {
    experiment: 'EXP-03',
    finding: 'A wide, shallow supervised organisation is the current default; a specialist split can help only where it returns independently useful value.',
    state: 'Published',
    home: '/journal/a-lead-is-a-consequence-window/'
  },
  {
    experiment: 'EXP-04',
    finding: 'Local closure is necessary but insufficient for parallel capacity. Startup and review cost can erase the overlap.',
    state: 'Published',
    home: '/journal/capacity-has-an-arrival-shape/'
  },
  {
    experiment: 'EXP-05',
    finding: 'Demand shape, causal supervision and evaluator validity all change what an apparent company outcome means.',
    state: 'Published',
    home: '/journal/capacity-has-an-arrival-shape/'
  },
  {
    experiment: 'EXP-06',
    finding: 'The first Restless-versus-Codex site comparison was inconclusive because both arms inherited the same visual identity.',
    state: 'Published',
    home: '/journal/a-comparison-needs-a-cleaner-start/'
  },
  {
    experiment: 'EXP-07',
    finding: 'The greenfield comparison has clean route and accessibility evidence, but the blind owner decision and arm identity remain sealed.',
    state: 'Deferred',
    home: '/research/corpus/',
    reason: 'Publication would reveal or infer a result before the owner has made the blind decision. The method is public; the winner is not.'
  }
];
