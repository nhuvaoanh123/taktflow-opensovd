<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- UC09 — ECU card header with HW/SW version, serial, VIN (FR-3.2) -->
<script lang="ts">
	import type { EcuId } from '$lib/types/sovd';
	import { CANNED_COMPONENTS } from '$lib/api/sovdClient';

	interface Props {
		componentId: EcuId;
	}

	let { componentId }: Props = $props();

	const comp = $derived(CANNED_COMPONENTS.find((c) => c.id === componentId));
</script>

{#if comp}
	<div class="rounded-lg border border-border bg-card p-3 text-xs">
		<h3 class="mb-2 font-semibold">{comp.label}</h3>
		<dl class="grid grid-cols-2 gap-x-3 gap-y-0.5">
			<dt class="text-muted-foreground">HW Rev</dt>
			<dd class="font-mono">{comp.hwVersion}</dd>

			<dt class="text-muted-foreground">SW Rev</dt>
			<dd class="font-mono">{comp.swVersion}</dd>

			<dt class="text-muted-foreground">Serial</dt>
			<dd class="font-mono">{comp.serial}</dd>

			<dt class="text-muted-foreground">VIN</dt>
			<dd class="font-mono">{comp.vin}</dd>
		</dl>
	</div>
{/if}
