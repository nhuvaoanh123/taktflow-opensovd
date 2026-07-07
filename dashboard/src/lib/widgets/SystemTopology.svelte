<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Reference strip: how a request and a fault travel through the SIL. -->
<script lang="ts">
	import { GitBranch } from 'lucide-svelte';

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

<section class="rounded-lg border border-border bg-card p-5 shadow-sm">
	<h3 class="flex items-center gap-2 text-base font-semibold">
		<span class="flex h-6 w-6 items-center justify-center rounded-md bg-indigo-50 text-indigo-600">
			<GitBranch class="h-3.5 w-3.5" />
		</span>
		Topology
	</h3>
	<p class="mb-3 mt-0.5 text-xs text-muted-foreground">
		The path your requests take through the bench, and the pipeline a fault passes before it
		lands in the DTC store above.
	</p>
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
</section>
