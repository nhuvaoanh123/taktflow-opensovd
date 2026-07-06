<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC08 - Component discovery top bar with capability pills (FR-3.1, FR-3.4) -->
<script lang="ts">
	import { onMount } from 'svelte';

	import { CANNED_COMPONENTS, listComponents } from '$lib/api/sovdClient';
	import type { EcuId, SovdComponent } from '$lib/types/sovd';

	interface Props {
		onSelect?: (id: EcuId) => void;
		selectedId?: EcuId;
	}

	let { onSelect, selectedId }: Props = $props();

	let components = $state<SovdComponent[]>([...CANNED_COMPONENTS]);

	onMount(() => {
		void load();
	});

	async function load() {
		components = await listComponents();
	}

	const CAP_COLOR: Record<string, string> = {
		faults: 'bg-red-800 text-red-200',
		operations: 'bg-blue-800 text-blue-200',
		data: 'bg-green-800 text-green-200',
		modes: 'bg-purple-800 text-purple-200'
	};

	const SOURCE_COLOR: Record<string, string> = {
		local: 'border-slate-500 bg-slate-800 text-slate-100',
		cda: 'border-emerald-600 bg-emerald-950 text-emerald-100',
		dfm: 'border-cyan-600 bg-cyan-950 text-cyan-100',
		unknown: 'border-zinc-600 bg-zinc-900 text-zinc-100'
	};
</script>

<div class="flex flex-wrap gap-3">
	{#each components as comp (comp.id)}
		<button
			onclick={() => onSelect?.(comp.id)}
			class="flex flex-col gap-1 rounded-lg border px-3 py-2 text-left transition-colors
				{selectedId === comp.id
				? 'border-primary bg-primary/10'
				: 'border-border bg-card hover:bg-accent/20'}"
		>
			<span class="text-sm font-bold">{comp.label}</span>
			<div class="flex flex-wrap items-center gap-1 text-[10px]">
				<span class="rounded border px-1.5 py-0.5 font-semibold uppercase {SOURCE_COLOR[comp.source]}">
					{comp.source}
				</span>
				{#if comp.logicalAddress}
					<span class="font-mono text-muted-foreground">{comp.logicalAddress}</span>
				{/if}
				{#if comp.state}
					<span class="text-muted-foreground">{comp.state}</span>
				{/if}
			</div>
			<span class="text-[10px] text-muted-foreground">S/N: {comp.serial}</span>
			<div class="flex flex-wrap gap-1">
				{#each comp.capabilities as cap (cap)}
					<span class="rounded px-1.5 py-0.5 text-[10px] font-semibold {CAP_COLOR[cap]}">
						{cap}
					</span>
				{/each}
			</div>
		</button>
	{/each}
</div>
