export const brand = {
  name: 'Restless',
  siteTitle: 'Restless | A company that keeps working',
  description:
    'An autonomous company control plane that turns owner intent into inspected, attributable work.',
  repository: 'https://github.com/BlueprintLabIO/restless'
} as const;

export const navigation = [
  { href: '/product/', label: 'Product' },
  { href: '/how-it-works/', label: 'How it works' },
  { href: '/research/', label: 'Research' },
  { href: '/compare/', label: 'Compare' },
  { href: '/findings/', label: 'Findings' }
] as const;
