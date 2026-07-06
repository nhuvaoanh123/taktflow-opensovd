<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC16 - Append-only audit log stream (SEC-3.1) -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { CANNED_AUDIT, getAuditLog } from '$lib/api/sovdClient';
	import type { AuditEntry } from '$lib/types/sovd';

	interface Props {
		extraEntries?: AuditEntry[];
	}

	let { extraEntries = [] }: Props = $props();

	let entries = $state<AuditEntry[]>([...CANNED_AUDIT]);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function load() {
		entries = await getAuditLog(50);
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

	const RESULT_COLOR = { ok: 'text-emerald-700', denied: 'text-red-700', error: 'text-orange-700' };
</script>

<div class="rounded-md border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		Audit log
	</h3>
	<div class="max-h-32 overflow-y-auto space-y-px font-mono text-[10px]">
		{#each allEntries as entry, i (i)}
			<div class="flex gap-2 border-b border-border/50 py-0.5">
				<span class="shrink-0 tabular-nums text-muted-foreground">
					{new Date(entry.timestamp).toLocaleTimeString()}
				</span>
				<span class="shrink-0 text-slate-700">{entry.actor}</span>
				<span class="shrink-0 font-semibold">{entry.action}</span>
				<span class="grow text-muted-foreground">{entry.target}</span>
				<span class="shrink-0 {RESULT_COLOR[entry.result]}">{entry.result}</span>
			</div>
		{/each}
	</div>
</div>
