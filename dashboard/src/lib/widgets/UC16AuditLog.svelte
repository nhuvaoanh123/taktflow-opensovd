<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC16 - Append-only audit log stream (SEC-3.1) -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { getAuditLog } from '$lib/api/sovdClient';
	import type { AuditEntry } from '$lib/types/sovd';

	interface Props {
		extraEntries?: AuditEntry[];
	}

	let { extraEntries = [] }: Props = $props();

	let entries = $state<AuditEntry[]>([]);
	let loading = $state(true);
	let unavailable = $state(false);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function load() {
		const loaded = await getAuditLog(50);
		unavailable = loaded === null;
		entries = loaded ?? [];
		loading = false;
	}

	onMount(() => {
		void load();
		timer = setInterval(() => {
			void load();
		}, 3000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	const allEntries = $derived([...extraEntries, ...entries].slice(0, 50));

	// Collapse consecutive repeats of the same action so polling traffic
	// reads as one row with a count instead of a wall of duplicates.
	const grouped = $derived(
		allEntries.reduce<Array<AuditEntry & { count: number }>>((groups, entry) => {
			const last = groups[groups.length - 1];
			if (
				last &&
				last.actor === entry.actor &&
				last.action === entry.action &&
				last.target === entry.target &&
				last.result === entry.result
			) {
				last.count += 1;
			} else {
				groups.push({ ...entry, count: 1 });
			}
			return groups;
		}, [])
	);

	const RESULT_COLOR = { ok: 'text-emerald-700', denied: 'text-red-700', error: 'text-orange-700' };
</script>

<div class="rounded-lg border border-border bg-card p-5 shadow-sm">
	<h3 class="mb-3 text-base font-semibold">Audit log</h3>
	{#if allEntries.length === 0}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{#if loading}
				Loading audit log...
			{:else if unavailable}
				Audit route unavailable.
			{:else}
				No audit entries recorded.
			{/if}
		</p>
	{/if}
	<div class="max-h-40 overflow-y-auto space-y-px font-mono text-[11px]">
		{#each grouped as entry, i (i)}
			<div class="flex gap-2 border-b border-border/50 py-0.5">
				<span class="shrink-0 tabular-nums text-muted-foreground">
					{new Date(entry.timestamp).toLocaleTimeString()}
				</span>
				<span class="shrink-0 text-slate-700">{entry.actor}</span>
				<span class="shrink-0 font-semibold">{entry.action}</span>
				<span class="grow truncate text-muted-foreground">{entry.target}</span>
				{#if entry.count > 1}
					<span class="shrink-0 text-muted-foreground">&times;{entry.count}</span>
				{/if}
				<span class="shrink-0 {RESULT_COLOR[entry.result]}">{entry.result}</span>
			</div>
		{/each}
	</div>
</div>
