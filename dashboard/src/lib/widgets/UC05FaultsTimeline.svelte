<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC05 - Aggregated DTC timeline across all ECUs (FR-1.5) -->
<script lang="ts">
	import { CANNED_DTCS, listAllFaults } from '$lib/api/sovdClient';
	import type { DtcEntry } from '$lib/types/sovd';

	interface Props {
		extraFaults?: DtcEntry[];
		refreshNonce?: number;
	}

	let { extraFaults = [], refreshNonce = 0 }: Props = $props();

	let baseFaults = $state<DtcEntry[]>([...CANNED_DTCS]);

	$effect(() => {
		void load(refreshNonce);
	});

	async function load(_refreshNonce: number) {
		baseFaults = await listAllFaults();
	}

	const all = $derived(
		[...baseFaults, ...extraFaults].sort(
			(left, right) => new Date(right.lastSeen).getTime() - new Date(left.lastSeen).getTime()
		)
	);

	const SEV_DOT: Record<string, string> = {
		critical: 'bg-red-600',
		high: 'bg-orange-500',
		medium: 'bg-amber-500',
		low: 'bg-slate-400'
	};

	function rel(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const seconds = Math.floor(diff / 1000);
		if (seconds < 60) return `${seconds}s ago`;
		if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
		return `${Math.floor(seconds / 3600)}h ago`;
	}
</script>

<div class="rounded-md border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		Fault timeline ({all.length})
	</h3>
	<ol class="space-y-0.5">
		{#each all as dtc (dtc.id)}
			<li class="flex items-start gap-2 border-b border-border/50 py-1 text-xs last:border-b-0">
				<span class="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full {SEV_DOT[dtc.severity]}"></span>
				<span class="w-16 shrink-0 font-mono font-semibold">{dtc.code}</span>
				<span class="grow truncate text-muted-foreground">{dtc.description}</span>
				<span class="shrink-0 text-[10px] uppercase text-muted-foreground">{dtc.component}</span>
				<span class="shrink-0 text-[10px] tabular-nums text-muted-foreground">{rel(dtc.lastSeen)}</span>
			</li>
		{/each}
	</ol>
</div>
