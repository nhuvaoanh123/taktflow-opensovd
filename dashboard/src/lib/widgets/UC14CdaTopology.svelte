<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- CDA routing topology. -->
<script lang="ts">
	const NODES = [
		{ id: 'tester', label: 'Tester', sub: 'public HTTPS' },
		{ id: 'gateway', label: 'Gateway', sub: 'sovd-main' },
		{ id: 'cda', label: 'CDA', sub: 'classic adapter' },
		{ id: 'transport', label: 'DoIP', sub: 'sim network' },
		{ id: 'ecu', label: 'ECU simulator', sub: 'CVC/FLX' }
	];
</script>

<div class="rounded-md border border-border bg-card p-3">
	<h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		CDA routing
	</h3>
	<div class="grid gap-2 sm:grid-cols-5">
		{#each NODES as node, i (node.id)}
			<div class="rounded border border-border bg-muted/40 px-2 py-2 text-center text-xs">
				<div class="mb-1 flex items-center justify-between gap-2">
					<span class="font-medium">{node.label}</span>
					<span class="font-mono text-[10px] text-muted-foreground">{i + 1}</span>
				</div>
				<p class="text-[10px] text-muted-foreground">{node.sub}</p>
			</div>
		{/each}
	</div>
</div>
