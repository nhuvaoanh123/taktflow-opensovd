<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC16 — Append-only audit log stream (SEC-3.1) -->
<script lang="ts">
	import type { AuditEntry } from '$lib/types/sovd';
	import { CANNED_AUDIT } from '$lib/api/sovdClient';
	import { onMount, onDestroy } from 'svelte';

	interface Props {
		extraEntries?: AuditEntry[];
	}

	let { extraEntries = [] }: Props = $props();

	let entries = $state<AuditEntry[]>([...CANNED_AUDIT]);
	let timer: ReturnType<typeof setInterval> | null = null;

	const ACTIONS_STUB = [
		'READ_DTC', 'POLL_ROUTINE', 'GET_VERSION', 'LIST_COMPONENTS'
	];
	const ACTORS_STUB = ['tester-01', 'tester-02', 'api-monitor'];

	onMount(() => {
		// Simulate new entries arriving
		timer = setInterval(() => {
			const action = ACTIONS_STUB[Math.floor(Math.random() * ACTIONS_STUB.length)];
			const actor = ACTORS_STUB[Math.floor(Math.random() * ACTORS_STUB.length)];
			const newEntry: AuditEntry = {
				timestamp: new Date().toISOString(),
				actor,
				action,
				target: (['cvc', 'sc', 'bcm'] as const)[Math.floor(Math.random() * 3)],
				result: 'ok'
			};
			entries = [newEntry, ...entries].slice(0, 50);
		}, 3000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	const allEntries = $derived([...extraEntries, ...entries].slice(0, 50));

	const RESULT_COLOR = { ok: 'text-green-400', denied: 'text-red-400', error: 'text-orange-400' };
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Audit Log — SEC-3.1 (append-only)
	</h3>
	<div class="max-h-32 overflow-y-auto font-mono text-[10px] space-y-px">
		{#each allEntries as entry, i (i)}
			<div class="flex gap-2 border-b border-border/30 py-0.5">
				<span class="shrink-0 tabular-nums text-muted-foreground"
					>{new Date(entry.timestamp).toLocaleTimeString()}</span
				>
				<span class="shrink-0 text-blue-300">{entry.actor}</span>
				<span class="shrink-0 font-semibold">{entry.action}</span>
				<span class="grow text-muted-foreground">{entry.target}</span>
				<span class="shrink-0 {RESULT_COLOR[entry.result]}">{entry.result}</span>
			</div>
		{/each}
	</div>
</div>
