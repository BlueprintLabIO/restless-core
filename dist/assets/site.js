const menu = document.querySelector('.menu');
const nav = document.querySelector('#nav');
menu?.addEventListener('click', () => {
  const open = menu.getAttribute('aria-expanded') === 'true';
  menu.setAttribute('aria-expanded', String(!open));
  nav.classList.toggle('open', !open);
});

const examples = {
  launch: {
    mission: 'Find the weakest part of our launch plan and repair what can be repaired.',
    decision: 'Keep Friday’s date with a smaller first cohort?'
  },
  research: {
    mission: 'Test whether this market thesis survives current primary-source evidence.',
    decision: 'Fund a real-workload replication next?'
  },
  operations: {
    mission: 'Reconcile the failed supplier run and preserve every safe recovery step.',
    decision: 'Approve the revised production order?'
  }
};
document.querySelectorAll('[data-example]').forEach(button => button.addEventListener('click', () => {
  const example = examples[button.dataset.example];
  document.querySelector('[data-mission]').textContent = example.mission;
  document.querySelector('.decision strong').textContent = example.decision;
  document.querySelector('[data-result]').textContent = 'Everything around this decision is prepared. Nothing consequential has been sent.';
  document.querySelectorAll('[data-example]').forEach(item => item.classList.toggle('active', item === button));
}));
document.querySelectorAll('[data-choice]').forEach(button => button.addEventListener('click', () => {
  const result = document.querySelector('[data-result]');
  result.textContent = button.dataset.choice === 'yes'
    ? 'Decision staged in this demonstration. No external action occurs.'
    : 'Trade-offs preserved: timing, cohort size and the repaired onboarding path.';
}));
