import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'astro/zod';

const journal = defineCollection({
  loader: glob({ base: './src/content/journal', pattern: '**/*.md' }),
  schema: z.object({
    title: z.string(),
    deck: z.string(),
    thesis: z.string(),
    publishedAt: z.coerce.date(),
    order: z.number().int().positive(),
    readTime: z.string(),
    status: z.enum(['Accepted direction', 'Provisional evidence', 'Open question']),
    experiments: z.array(z.string()),
    evidence: z.array(
      z.object({
        label: z.string(),
        locator: z.string(),
        scope: z.string()
      })
    )
  })
});

export const collections = { journal };
