<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC14 — CDA topology: Tester → Gateway → CDA → Proxy → ECU (FR-5.1, FR-5.2) -->
<script lang="ts">
	const NODES = [
		{ id: 'tester', label: 'Tester', sub: ':21002', color: 'border-blue-500 bg-blue-900 text-blue-100' },
		{ id: 'gateway', label: 'Gateway', sub: 'sovd-main', color: 'border-slate-500 bg-slate-800 text-slate-100' },
		{ id: 'cda', label: 'CDA', sub: 'classic-diag', color: 'border-purple-500 bg-purple-900 text-purple-100' },
		{ id: 'proxy', label: 'CAN-DoIP', sub: ':13401', color: 'border-yellow-600 bg-yellow-900 text-yellow-100' },
		{ id: 'ecu', label: 'ECU', sub: '0x7E0 CVC', color: 'border-green-500 bg-green-900 text-green-100' }
	];
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		CDA Routing Topology
	</h3>
	<div class="flex items-center gap-1 overflow-x-auto">
		{#each NODES as node, i (node.id)}
			<div
				class="flex min-w-[64px] flex-col items-center rounded border px-2 py-2 text-center text-xs {node.color}"
			>
				<span class="font-semibold">{node.label}</span>
				<span class="text-[10px] opacity-75">{node.sub}</span>
			</div>
			{#if i < NODES.length - 1}
				<span class="shrink-0 text-sm text-muted-foreground">→</span>
			{/if}
		{/each}
	</div>
	<p class="mt-2 text-[10px] text-muted-foreground">
		Legacy UDS traffic bridged over DoIP — ADR-0015 CDA adapter
	</p>
</div>
