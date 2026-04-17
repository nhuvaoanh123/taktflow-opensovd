<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC18 — Gateway routing / backend registry topology (FR-6.1, FR-6.2) -->
<script lang="ts">
	import type { GatewayBackend } from '$lib/types/sovd';
	import { CANNED_BACKENDS } from '$lib/api/sovdClient';

	const backends: GatewayBackend[] = CANNED_BACKENDS;
</script>

<div class="rounded-lg border border-border bg-card p-3">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
		Gateway Routing ({backends.length} backends)
	</h3>
	<table class="w-full text-xs">
		<thead>
			<tr class="border-b border-border">
				<th class="py-1 text-left font-medium text-muted-foreground">Backend</th>
				<th class="py-1 text-left font-medium text-muted-foreground">Address</th>
				<th class="py-1 text-left font-medium text-muted-foreground">Proto</th>
				<th class="py-1 text-right font-medium text-muted-foreground">Latency</th>
				<th class="py-1 text-right font-medium text-muted-foreground">Status</th>
			</tr>
		</thead>
		<tbody>
			{#each backends as b (b.id)}
				<tr class="border-b border-border/40">
					<td class="py-1 font-mono">{b.id}</td>
					<td class="py-1 font-mono text-muted-foreground">{b.address}</td>
					<td class="py-1 uppercase">{b.protocol}</td>
					<td class="py-1 text-right tabular-nums">
						{b.reachable ? `${b.latencyMs} ms` : '—'}
					</td>
					<td class="py-1 text-right">
						{#if b.reachable}
							<span class="text-green-400">● up</span>
						{:else}
							<span class="text-red-400">● down</span>
						{/if}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
