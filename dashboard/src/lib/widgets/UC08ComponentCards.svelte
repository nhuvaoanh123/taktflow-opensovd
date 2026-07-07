<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC08 - Component discovery top bar with capability pills (FR-3.1, FR-3.4) -->
<script lang="ts">
	import { onMount } from 'svelte';

	import { listComponents } from '$lib/api/sovdClient';
	import type { EcuId, SovdComponent } from '$lib/types/sovd';

	interface Props {
		onSelect?: (id: EcuId) => void;
		selectedId?: EcuId;
		onLoaded?: (count: number | null) => void;
	}

	let { onSelect, selectedId, onLoaded }: Props = $props();

	let components = $state<SovdComponent[]>([]);
	let loading = $state(true);
	let unavailable = $state(false);

	onMount(() => {
		void load();
	});

	async function load() {
		loading = true;
		try {
			const discovered = await listComponents();
			unavailable = discovered === null;
			components = discovered ?? [];
			onLoaded?.(discovered === null ? null : discovered.length);
		} finally {
			loading = false;
		}
	}

	const CAP_COLOR: Record<string, string> = {
		faults: 'border-slate-200 bg-slate-50 text-slate-600',
		operations: 'border-slate-200 bg-slate-50 text-slate-600',
		data: 'border-slate-200 bg-slate-50 text-slate-600',
		modes: 'border-slate-200 bg-slate-50 text-slate-600'
	};

	// Source identity carries a fixed tint per origin; the text label is the
	// identity channel, the tint is reinforcement.
	const SOURCE_COLOR: Record<string, string> = {
		local: 'border-slate-300 bg-slate-100 text-slate-700',
		cda: 'border-indigo-200 bg-indigo-50 text-indigo-700',
		dfm: 'border-violet-200 bg-violet-50 text-violet-700',
		unknown: 'border-slate-200 bg-slate-50 text-slate-500'
	};
</script>

{#if components.length === 0}
	<p class="rounded-lg border border-border bg-card px-3 py-4 text-center text-xs text-muted-foreground shadow-sm">
		{#if loading}
			Discovering components...
		{:else if unavailable}
			Component discovery unavailable — /sovd/v1/components did not respond.
		{:else}
			No components discovered.
		{/if}
	</p>
{:else}
<div class="grid gap-3 md:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-6">
	{#each components as comp (comp.id)}
		<button
			onclick={() => onSelect?.(comp.id)}
			class="flex min-h-28 flex-col gap-1.5 rounded-lg border px-3 py-2.5 text-left shadow-sm transition-colors
				{selectedId === comp.id
				? 'border-indigo-600 bg-indigo-50/60 ring-1 ring-indigo-600'
				: 'border-border bg-card hover:border-indigo-300'}"
		>
			<span class="truncate text-sm font-semibold">{comp.label}</span>
			<div class="flex flex-wrap items-center gap-1 text-[10px] text-muted-foreground">
				<span class="rounded border px-1.5 py-0.5 font-medium uppercase {SOURCE_COLOR[comp.source]}">
					{comp.source}
				</span>
				{#if comp.logicalAddress}
					<span class="font-mono text-muted-foreground">{comp.logicalAddress}</span>
				{/if}
				{#if comp.state}
					<span class="text-muted-foreground">{comp.state}</span>
				{/if}
			</div>
			{#if comp.serial}
				<span class="truncate text-[10px] text-muted-foreground">S/N {comp.serial}</span>
			{/if}
			<div class="flex flex-wrap gap-1">
				{#each comp.capabilities as cap (cap)}
					<span class="rounded border px-1.5 py-0.5 text-[10px] font-medium {CAP_COLOR[cap]}">
						{cap}
					</span>
				{/each}
			</div>
		</button>
	{/each}
</div>
{/if}
