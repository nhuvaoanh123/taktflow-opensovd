<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC16 - Append-only audit log stream (SEC-3.1) -->
<script lang="ts">
	import { Terminal } from 'lucide-svelte';
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

	const RESULT_COLOR = { ok: 'text-emerald-400', denied: 'text-red-400', error: 'text-orange-400' };
</script>

<div class="rounded-lg border border-slate-800 bg-slate-900 p-5 text-slate-300 shadow-sm">
	<h3 class="flex items-center gap-2 text-base font-semibold text-white">
		<span class="flex h-6 w-6 items-center justify-center rounded-md bg-slate-800 text-emerald-400">
			<Terminal class="h-3.5 w-3.5" />
		</span>
		Audit log
	</h3>
	<p class="mb-3 mt-0.5 text-xs text-slate-400">
		Every API call the gateway serves — including the ones this page is making right now.
	</p>
	{#if allEntries.length === 0}
		<p class="py-2 text-center text-xs text-slate-400">
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
			<div class="flex gap-2 border-b border-slate-800 py-0.5">
				<span class="shrink-0 tabular-nums text-slate-500">
					{new Date(entry.timestamp).toLocaleTimeString()}
				</span>
				<span class="shrink-0 text-slate-400">{entry.actor}</span>
				<span class="shrink-0 font-semibold text-slate-100">{entry.action}</span>
				<span class="grow truncate text-slate-500">{entry.target}</span>
				{#if entry.count > 1}
					<span class="shrink-0 text-slate-400">&times;{entry.count}</span>
				{/if}
				<span class="shrink-0 {RESULT_COLOR[entry.result]}">{entry.result}</span>
			</div>
		{/each}
	</div>
</div>
