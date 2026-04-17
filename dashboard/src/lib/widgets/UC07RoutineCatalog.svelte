<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC07 — Routine catalog discovery per ECU (FR-2.4) -->
<script lang="ts">
	import type { EcuId, RoutineEntry } from '$lib/types/sovd';
	import { CANNED_ROUTINES } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	const routines = $derived(CANNED_ROUTINES.filter((r) => r.component === componentId));
	let selected = $state<RoutineEntry | null>(null);
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Routine Catalog — {componentId.toUpperCase()}
	</h3>
	{#if routines.length === 0}
		<p class="text-xs text-muted-foreground">No routines registered for this ECU.</p>
	{:else}
		<select
			class="w-full rounded border border-border bg-background px-2 py-1 text-xs text-foreground"
			onchange={(e) => {
				const id = (e.target as HTMLSelectElement).value;
				selected = routines.find((r) => r.id === id) ?? null;
			}}
		>
			<option value="">— select routine —</option>
			{#each routines as rt (rt.id)}
				<option value={rt.id}>{rt.name}</option>
			{/each}
		</select>
		{#if selected}
			<div class="mt-2 rounded bg-muted/30 p-2 text-xs">
				<p><span class="text-muted-foreground">ID:</span> <span class="font-mono">{selected.id}</span></p>
				<p><span class="text-muted-foreground">Status:</span> {selected.status}</p>
				{#if selected.lastResult}
					<p><span class="text-muted-foreground">Last result:</span> {selected.lastResult}</p>
				{/if}
			</div>
		{/if}
	{/if}
</div>
