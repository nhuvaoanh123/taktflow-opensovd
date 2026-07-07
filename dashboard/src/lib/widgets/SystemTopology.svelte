<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Reference strip: how a request and a fault travel through the SIL. -->
<script lang="ts">
	import { GitBranch } from 'lucide-svelte';

	import Panel from './Panel.svelte';

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

<Panel
	title="Topology"
	meta="how requests and faults travel"
	hint="The path your requests take through the bench, and the pipeline a fault passes before it is recorded in the DTC store."
	open={false}
	chip="bg-indigo-50 text-indigo-600"
>
	{#snippet icon()}<GitBranch class="h-3.5 w-3.5" />{/snippet}
	<div class="space-y-2.5">
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
</Panel>
