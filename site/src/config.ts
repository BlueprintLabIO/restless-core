export const brand = {
  name: 'Restless',
  siteTitle: 'Restless | The company handles the work',
  description:
    'An autonomous company control plane that turns owner intent into inspected, attributable work.',
  repository: 'https://github.com/BlueprintLabIO/restless'
} as const;

export const navigation = [
  { href: '/product/', label: 'Product' },
  { href: '/research/', label: 'Research' },
  { href: '/compare/', label: 'Compare' },
  { href: '/findings/', label: 'Findings' }
] as const;
