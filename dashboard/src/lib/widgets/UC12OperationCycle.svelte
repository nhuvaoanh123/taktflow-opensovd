<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Operation-cycle status from the public health endpoint when reported. -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { getGatewayHealth } from '$lib/api/sovdClient';

	const STATES = ['Idle', 'Running', 'Evaluating', 'Complete'];

	let reported = $state<string | null>(null);
	let timer: ReturnType<typeof setInterval> | null = null;

	async function load() {
		reported = (await getGatewayHealth())?.operationCycle ?? null;
	}

	onMount(() => {
		void load();
		timer = setInterval(() => {
			void load();
		}, 5000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	function isCurrent(state: string): boolean {
		return reported?.toLowerCase() === state.toLowerCase();
	}
</script>

<div class="rounded-md border border-border bg-card p-3">
	<div class="mb-3 flex items-center justify-between gap-2">
		<h3 class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
			Operation cycle
		</h3>
		<span class="text-[10px] text-muted-foreground">
			{reported ?? 'Not reported'}
		</span>
	</div>
	<div class="grid gap-2 sm:grid-cols-4">
		{#each STATES as state (state)}
			<div
				class="rounded border px-2 py-2 text-center text-xs
					{isCurrent(state)
					? 'border-slate-900 bg-slate-50 text-slate-900'
					: 'border-border bg-muted/30 text-muted-foreground'}"
			>
				<span class="font-medium">{state}</span>
			</div>
		{/each}
	</div>
</div>
