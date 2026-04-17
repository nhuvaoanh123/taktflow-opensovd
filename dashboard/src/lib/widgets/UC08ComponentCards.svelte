<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC08 — Component discovery top bar with capability pills (FR-3.1, FR-3.4) -->
<script lang="ts">
	import type { SovdComponent, EcuId } from '$lib/types/sovd';
	import { CANNED_COMPONENTS } from '$lib/api/sovdClient';

	interface Props {
		onSelect?: (id: EcuId) => void;
		selectedId?: EcuId;
	}

	let { onSelect, selectedId }: Props = $props();

	const components: SovdComponent[] = CANNED_COMPONENTS;

	const CAP_COLOR: Record<string, string> = {
		faults: 'bg-red-800 text-red-200',
		operations: 'bg-blue-800 text-blue-200',
		data: 'bg-green-800 text-green-200',
		modes: 'bg-purple-800 text-purple-200'
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
