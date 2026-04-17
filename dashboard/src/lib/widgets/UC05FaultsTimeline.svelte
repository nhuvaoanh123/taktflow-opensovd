<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC05 — Aggregated DTC timeline across all ECUs (FR-1.5) -->
<script lang="ts">
	import type { DtcEntry } from '$lib/types/sovd';
	import { CANNED_DTCS } from '$lib/api/sovdClient';

	// Accept live faults from parent (may be augmented by WS events)
	interface Props {
		extraFaults?: DtcEntry[];
	}

	let { extraFaults = [] }: Props = $props();

	const all = $derived([...CANNED_DTCS, ...extraFaults].sort(
		(a, b) => new Date(b.lastSeen).getTime() - new Date(a.lastSeen).getTime()
	));

	const SEV_DOT: Record<string, string> = {
		critical: 'bg-red-500',
		high: 'bg-orange-500',
		medium: 'bg-yellow-400',
		low: 'bg-slate-400'
	};

	function rel(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const s = Math.floor(diff / 1000);
		if (s < 60) return `${s}s ago`;
		if (s < 3600) return `${Math.floor(s / 60)}m ago`;
		return `${Math.floor(s / 3600)}h ago`;
	}
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Aggregated Fault Timeline ({all.length} events)
	</h3>
	<ol class="space-y-1">
		{#each all as dtc (dtc.id)}
			<li class="flex items-start gap-2 text-xs">
				<span class="mt-1 h-2 w-2 shrink-0 rounded-full {SEV_DOT[dtc.severity]}"></span>
				<span class="font-mono font-semibold">{dtc.code}</span>
				<span class="grow truncate text-muted-foreground">{dtc.description}</span>
				<span class="shrink-0 text-[10px] uppercase text-muted-foreground"
					>{dtc.component}</span
				>
				<span class="shrink-0 text-[10px] tabular-nums text-muted-foreground"
					>{rel(dtc.lastSeen)}</span
				>
			</li>
		{/each}
	</ol>
</div>
