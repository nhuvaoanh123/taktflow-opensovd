<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC05 - Aggregated DTC timeline across all ECUs (FR-1.5) -->
<script lang="ts">
	import { Activity } from 'lucide-svelte';

	import { listAllFaults } from '$lib/api/sovdClient';
	import type { DtcEntry } from '$lib/types/sovd';
	import Panel from './Panel.svelte';

	interface Props {
		extraFaults?: DtcEntry[];
		refreshNonce?: number;
		onCount?: (count: number | null) => void;
	}

	let { extraFaults = [], refreshNonce = 0, onCount }: Props = $props();

	let baseFaults = $state<DtcEntry[]>([]);
	let loading = $state(true);
	let unavailable = $state(false);

	$effect(() => {
		void load(refreshNonce);
	});

	async function load(_refreshNonce: number) {
		loading = true;
		try {
			const faults = await listAllFaults();
			unavailable = faults === null;
			baseFaults = faults ?? [];
		} finally {
			loading = false;
		}
	}

	function timeMs(iso?: string): number {
		const parsed = iso ? Date.parse(iso) : Number.NaN;
		return Number.isFinite(parsed) ? parsed : 0;
	}

	const all = $derived(
		[...baseFaults, ...extraFaults].sort((left, right) => timeMs(right.lastSeen) - timeMs(left.lastSeen))
	);

	$effect(() => {
		// Report null while loading or when the routes are unavailable so the
		// hero tile shows -- instead of a false zero.
		onCount?.(loading || (unavailable && all.length === 0) ? null : all.length);
	});

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

<Panel
	title="Fault feed"
	meta={`all components · ${all.length}`}
	hint="Every fault on the bench right now, newest first — dot color is severity. New events stream in live."
	chip="bg-amber-50 text-amber-600"
>
	{#snippet icon()}<Activity class="h-3.5 w-3.5" />{/snippet}
	{#if all.length === 0}
		<p class="py-2 text-center text-xs text-muted-foreground">
			{#if loading}
				Loading fault timeline...
			{:else if unavailable}
				Fault routes unavailable.
			{:else}
				No faults reported.
			{/if}
		</p>
	{/if}
	<ol class="space-y-0.5">
		{#each all as dtc (dtc.id)}
			<li class="flex items-start gap-2.5 border-b border-border/50 py-1.5 text-sm last:border-b-0">
				<span class="mt-[7px] h-2 w-2 shrink-0 rounded-full {SEV_DOT[dtc.severity]}"></span>
				<span class="w-20 shrink-0 font-mono font-semibold">{dtc.code}</span>
				<span class="grow truncate text-muted-foreground">{dtc.description}</span>
				<span class="shrink-0 rounded border border-border bg-muted/50 px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">{dtc.component}</span>
				{#if dtc.lastSeen}
					<span class="shrink-0 pt-0.5 text-[11px] tabular-nums text-muted-foreground">{rel(dtc.lastSeen)}</span>
				{/if}
			</li>
		{/each}
	</ol>
</Panel>
