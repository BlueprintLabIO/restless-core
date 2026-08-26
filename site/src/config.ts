export const brand = {
  name: 'Restless',
  siteTitle: 'Restless | The company handles the work',
  description:
    'Set the direction. Restless carries the work to an inspected outcome and returns only the judgment that belongs to you.',
  repository: 'https://github.com/BlueprintLabIO/restless'
} as const;

export const navigation = [
  { href: '/product/', label: 'Product' },
  { href: '/how-it-works/', label: 'How it works' },
  { href: '/research/', label: 'Research' },
  { href: '/compare/', label: 'Compare' },
  { href: '/findings/', label: 'Findings' }
] as const;
