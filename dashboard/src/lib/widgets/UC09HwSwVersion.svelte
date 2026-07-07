<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC09 - ECU card header with HW/SW version, serial, VIN (FR-3.2) -->
<script lang="ts">
	import { getComponent } from '$lib/api/sovdClient';
	import type { EcuId, SovdComponent } from '$lib/types/sovd';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	let comp = $state<SovdComponent | null>(null);
	let loading = $state(true);

	$effect(() => {
		void load(componentId);
	});

	async function load(id: EcuId) {
		loading = true;
		try {
			comp = await getComponent(id);
		} finally {
			loading = false;
		}
	}
</script>

<div class="rounded-md border border-border bg-card p-3 text-xs">
	<h3 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
		Component details
	</h3>
	{#if comp}
		<p class="mb-2 font-semibold">{comp.label}</p>
		<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
			<dt class="text-muted-foreground">HW Rev</dt>
			<dd class="font-mono">{comp.hwVersion ?? '--'}</dd>

			<dt class="text-muted-foreground">SW Rev</dt>
			<dd class="font-mono">{comp.swVersion ?? '--'}</dd>

			<dt class="text-muted-foreground">Serial</dt>
			<dd class="font-mono">{comp.serial ?? '--'}</dd>

			<dt class="text-muted-foreground">VIN</dt>
			<dd class="font-mono">{comp.vin ?? '--'}</dd>
		</dl>
	{:else}
		<p class="text-muted-foreground">
			{loading
				? 'Loading component details...'
				: `Component detail route unavailable for ${componentId.toUpperCase()}.`}
		</p>
	{/if}
</div>
