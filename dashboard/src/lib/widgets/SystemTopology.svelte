<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Reference strip: how a request and a fault travel through the SIL. -->
<script lang="ts">
	import { ChevronDown, GitBranch } from 'lucide-svelte';

	const FLOWS = [
		{
			label: 'Request path',
			nodes: ['Tester (public HTTPS)', 'Gateway (sovd-main)', 'CDA', 'DoIP (sim network)', 'ECU simulator']
		},
		{
			label: 'Fault path',
			nodes: ['Fault ingest', 'Debounce', 'Operation-cycle binding', 'DTC store']
		}
	];
</script>

<details class="group rounded-lg border border-border bg-card shadow-sm">
	<summary class="flex cursor-pointer list-none items-center justify-between gap-2 p-5 [&::-webkit-details-marker]:hidden">
		<h3 class="flex items-center gap-2 text-base font-semibold">
			<span class="flex h-6 w-6 items-center justify-center rounded-md bg-indigo-50 text-indigo-600">
				<GitBranch class="h-3.5 w-3.5" />
			</span>
			Topology
			<span class="text-sm font-normal text-muted-foreground">— how requests and faults travel</span>
		</h3>
		<ChevronDown class="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-open:rotate-180" />
	</summary>
	<div class="space-y-2.5 px-5 pb-5">
		{#each FLOWS as flow (flow.label)}
			<div class="flex flex-wrap items-center gap-x-2 gap-y-1.5 text-sm">
				<span class="w-28 shrink-0 text-xs text-muted-foreground">{flow.label}</span>
				{#each flow.nodes as node, i (node)}
					{#if i > 0}
						<span class="text-muted-foreground/60" aria-hidden="true">&rarr;</span>
					{/if}
					<span
						class="flex items-center gap-1.5 rounded-md border border-border bg-muted/40 px-2 py-1 text-xs font-medium"
					>
						<span class="font-mono text-[10px] text-indigo-600">{i + 1}</span>
						{node}
					</span>
				{/each}
			</div>
		{/each}
	</div>
</details>
