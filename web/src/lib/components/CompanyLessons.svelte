<script lang="ts">
	/* Lessons: what the owner has ruled. Exec records one when the owner corrects something, and
	 * every agent it applies to reads it before working. Each row is the rule on one line, and under
	 * it how often the same problem came back and who the rule is for. Retiring one has an Undo. */
	import { formatDay } from '$lib/ui/time';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { Section, Item, Empty } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import {
		fetchLessons,
		lessonRecurrence,
		lessonScope,
		restoreLesson,
		retireLesson,
		type Lesson
	} from '$lib/model/lessons';

	let { companyId }: { companyId: string } = $props();

	const client = useQueryClient();
	const key = () => ['company-lessons', companyId];
	const lessons = createQuery(() => ({
		queryKey: key(),
		queryFn: () => fetchLessons(companyId),
		enabled: Boolean(companyId),
		staleTime: 30_000,
		retry: false
	}));
	let busy = $state('');

	async function retire(lesson: Lesson) {
		const company = companyId;
		busy = lesson.id;
		try {
			await retireLesson(company, lesson.id);
			await client.invalidateQueries({ queryKey: ['company-lessons', company] });
			toast('Lesson retired', {
				undo: async () => {
					try {
						await restoreLesson(company, lesson.id);
						await client.invalidateQueries({ queryKey: ['company-lessons', company] });
					} catch (cause) {
						toast(failureSentence(cause, 'That lesson was not brought back.'), {
							tone: 'failure'
						});
					}
				}
			});
		} catch (cause) {
			toast(failureSentence(cause, 'That lesson was not retired.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
</script>

<Section
	id="lessons"
	title="Lessons"
	count={lessons.data?.length}
	info="Rules Exec recorded from your corrections. Every agent a rule applies to reads it before working. When the same problem shows up again, the rule counts it."
>
	{#if lessons.isPending}
		<Skeleton label="Loading lessons" variant="list" count={2} />
	{:else if lessons.isError}
		<FailureNotice
			error={lessons.error}
			subject="lessons"
			variant="block"
			onretry={() => void lessons.refetch()}
		/>
	{:else}
		{#each lessons.data ?? [] as lesson (lesson.id)}
			<Item
				title={lesson.rule}
				note={`${lessonRecurrence(lesson)} · ${lessonScope(lesson)} · Recorded ${formatDay(lesson.created_at)}`}
			>
				{#snippet trailing()}
					<button
						class="btn small ghost"
						type="button"
						disabled={busy === lesson.id}
						onclick={() => void retire(lesson)}>Retire</button
					>
				{/snippet}
			</Item>
		{:else}
			<Empty compact title="None yet. Correct Exec in chat and the rule is kept here." />
		{/each}
	{/if}
</Section>
