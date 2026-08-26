import '@fontsource-variable/archivo';
import '@fontsource-variable/newsreader';
import './styles.css';

const menu = document.querySelector('[data-menu]');
const nav = document.querySelector('[data-nav]');
menu?.addEventListener('click', () => {
  const open = menu.getAttribute('aria-expanded') !== 'true';
  menu.setAttribute('aria-expanded', String(open));
  nav.dataset.open = String(open);
});

const scenarios = {
  launch: {
    ask: 'Decide which launch story earns the homepage.',
    steps: ['Lead frames the decision', 'Staff builds and probes two candidates', 'Lead rejects the weaker story'],
    result: 'Two live candidates are ready. Choose the one that sounds most like us.',
    artifact: 'Pre-positioned website previews'
  },
  accounts: {
    ask: 'Find the 40 accounts worth our attention this week.',
    steps: ['Lead defines qualification evidence', 'Staff closes independent account research', 'Lead checks the weakest edge cases'],
    result: '40 accounts qualify. Approve the prepared first outreach for the top six.',
    artifact: 'Ranked account brief and drafts'
  },
  support: {
    ask: 'Keep customer replies moving within our refund policy.',
    steps: ['Lead holds the policy and case charter', 'Staff resolves ordinary cases', 'Independent check catches an exception'],
    result: 'One refund exceeds your limit. The reply and supporting case are open for approval.',
    artifact: 'Exact case, prepared reply, approval'
  }
};

const scenarioButtons = document.querySelectorAll('[data-scenario]');
const ask = document.querySelector('[data-demo-ask]');
const steps = document.querySelector('[data-demo-steps]');
const result = document.querySelector('[data-demo-result]');
const artifact = document.querySelector('[data-demo-artifact]');
scenarioButtons.forEach((button) => button.addEventListener('click', () => {
  const item = scenarios[button.dataset.scenario];
  scenarioButtons.forEach((candidate) => candidate.setAttribute('aria-pressed', String(candidate === button)));
  if (!item || !ask || !steps || !result || !artifact) return;
  ask.textContent = item.ask;
  steps.innerHTML = item.steps.map((step) => `<li>${step}</li>`).join('');
  result.textContent = item.result;
  artifact.textContent = item.artifact;
}));

const observed = document.querySelectorAll('[data-observe]');
if ('IntersectionObserver' in window && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
  const observer = new IntersectionObserver((entries) => entries.forEach((entry) => {
    if (entry.isIntersecting) {
      entry.target.dataset.visible = 'true';
      observer.unobserve(entry.target);
    }
  }), { threshold: 0.12 });
  observed.forEach((element) => observer.observe(element));
} else {
  observed.forEach((element) => { element.dataset.visible = 'true'; });
}
