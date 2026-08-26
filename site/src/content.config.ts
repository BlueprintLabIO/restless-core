import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';

const findings = defineCollection({
  loader: glob({ base: './src/content/findings', pattern: '**/*.{md,mdx}' }),
  schema: z.object({
    title: z.string(),
    deck: z.string(),
    publishedAt: z.coerce.date(),
    order: z.number().int().positive(),
    readTime: z.string(),
    run: z.string(),
    finding: z.string(),
    status: z.enum(['Accepted', 'Provisional', 'Open'])
  })
});

export const collections = { findings };
