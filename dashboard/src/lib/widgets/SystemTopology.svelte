<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Quiet reference strip: how a request and a fault travel through the SIL. -->
<script lang="ts">
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

<section class="rounded-md border border-border bg-card p-4">
	<h3 class="mb-3 text-sm font-semibold">Topology</h3>
	<div class="space-y-2">
		{#each FLOWS as flow (flow.label)}
			<div class="flex flex-wrap items-baseline gap-x-3 gap-y-1 text-xs">
				<span class="w-24 shrink-0 text-muted-foreground">{flow.label}</span>
				{#each flow.nodes as node, i (node)}
					{#if i > 0}
						<span class="text-muted-foreground/50" aria-hidden="true">&rarr;</span>
					{/if}
					<span class="font-medium">{node}</span>
				{/each}
			</div>
		{/each}
	</div>
</section>
