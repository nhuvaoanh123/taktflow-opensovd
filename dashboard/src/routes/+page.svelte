<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Taktflow OpenSOVD - Live SIL Operations Dashboard -->
<script lang="ts">
	import { onDestroy, onMount } from 'svelte';

	import { getGatewayHealth } from '$lib/api/sovdClient';
	import { subscribe } from '$lib/api/wsClient';
	import type { DtcEntry, EcuId, GatewayHealth, TelemetryFrame } from '$lib/types/sovd';

	import UC01DtcList from '$lib/widgets/UC01DtcList.svelte';
	import UC02DtcDetail from '$lib/widgets/UC02DtcDetail.svelte';
	import UC03ClearFaults from '$lib/widgets/UC03ClearFaults.svelte';
	import UC04Pagination from '$lib/widgets/UC04Pagination.svelte';
	import UC05FaultsTimeline from '$lib/widgets/UC05FaultsTimeline.svelte';
	import UC06Operations from '$lib/widgets/UC06Operations.svelte';
	import UC08ComponentCards from '$lib/widgets/UC08ComponentCards.svelte';
	import UC09HwSwVersion from '$lib/widgets/UC09HwSwVersion.svelte';
	import UC10LiveDidReads from '$lib/widgets/UC10LiveDidReads.svelte';
	import UC15Session from '$lib/widgets/UC15Session.svelte';
	import UC16AuditLog from '$lib/widgets/UC16AuditLog.svelte';
	import UC18GatewayRouting from '$lib/widgets/UC18GatewayRouting.svelte';
	import UC19Historical from '$lib/widgets/UC19Historical.svelte';
	import SystemTopology from '$lib/widgets/SystemTopology.svelte';

	let selectedEcu = $state<EcuId>('cvc');
	let selectedDtc = $state<DtcEntry | null>(null);
	let dtcPage = $state(0);
	let showHistorical = $state(false);
	let filteredCount = $state(0);
	let faultRefreshNonce = $state(0);
	let liveFaults = $state<DtcEntry[]>([]);
	let health = $state<GatewayHealth | null>(null);
	let healthChecked = $state(false);
	let healthTimer: ReturnType<typeof setInterval> | null = null;

	const PAGE_SIZE = 5;
	const MUTATIONS_ENABLED = import.meta.env.VITE_SIL_MUTATIONS_ENABLED === 'true';

	async function pollHealth() {
		health = await getGatewayHealth();
		healthChecked = true;
	}

	onMount(() => {
		void pollHealth();
		healthTimer = setInterval(() => {
			void pollHealth();
		}, 15_000);
		return subscribe((frame: TelemetryFrame) => {
			if (frame.type !== 'dtc') {
				return;
			}
			liveFaults = [frame.payload as DtcEntry, ...liveFaults].slice(0, 50);
		});
	});

	onDestroy(() => {
		if (healthTimer) clearInterval(healthTimer);
	});
</script>

<svelte:head>
	<title>Taktflow OpenSOVD - Live SIL</title>
</svelte:head>

<UC02DtcDetail dtc={selectedDtc} onClose={() => (selectedDtc = null)} />

<div class="min-h-screen bg-background text-foreground">
	<header class="border-b border-border bg-card">
		<div class="mx-auto flex max-w-[1600px] flex-wrap items-center justify-between gap-4 px-6 py-4">
			<div>
				<h1 class="text-lg font-semibold tracking-tight">OpenSOVD SIL Operations</h1>
				<p class="text-xs text-muted-foreground">
					Public simulator environment — sovd-main, CDA, ECU simulator, MQTT
				</p>
			</div>
			<div class="flex flex-wrap items-center gap-5">
				<nav class="flex items-center gap-4 text-xs font-medium text-muted-foreground">
					<a href="https://taktflow-systems.com/" class="hover:text-foreground">Taktflow Systems</a>
					<a href="/sovd/" class="hover:text-foreground">Engineering spec</a>
					<a href="/sovd/grafana/" class="hover:text-foreground">Grafana</a>
				</nav>
				{#if health}
					<span
						class="flex items-center gap-2 rounded-full border border-border bg-background px-3 py-1 text-xs"
					>
						<span class="h-2 w-2 rounded-full bg-emerald-600"></span>
						<span>API healthy</span>
						<span class="text-muted-foreground">v{health.version} · {health.latencyMs} ms</span>
					</span>
				{:else if healthChecked}
					<span
						class="flex items-center gap-2 rounded-full border border-border bg-background px-3 py-1 text-xs"
					>
						<span class="h-2 w-2 rounded-full bg-red-600"></span>
						<span>API unreachable</span>
					</span>
				{/if}
			</div>
		</div>
	</header>

	<main class="mx-auto flex max-w-[1600px] flex-col gap-6 px-6 py-5">
		<section class="space-y-2">
			<div class="flex flex-wrap items-baseline justify-between gap-2">
				<h2 class="text-sm font-semibold">Components</h2>
				<span class="text-xs text-muted-foreground">
					{MUTATIONS_ENABLED ? 'Operator controls enabled' : 'Public read-only mode'}
				</span>
			</div>
			<UC08ComponentCards
				selectedId={selectedEcu}
				onSelect={(id) => {
					selectedEcu = id;
					dtcPage = 0;
				}}
			/>
		</section>

		<div class="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_minmax(0,1fr)]">
			<!-- Faults -->
			<div class="flex flex-col gap-6">
				<section class="rounded-md border border-border bg-card p-4">
					<UC01DtcList
						componentId={selectedEcu}
						page={dtcPage}
						pageSize={PAGE_SIZE}
						refreshNonce={faultRefreshNonce}
						onSelect={(dtc) => (selectedDtc = dtc)}
						onPage={(pageNumber) => (dtcPage = pageNumber)}
						onTotalChange={(total) => (filteredCount = total)}
					/>
					<div class="mt-3 flex flex-wrap items-center justify-between gap-2 border-t border-border pt-3">
						<UC03ClearFaults
							componentId={selectedEcu}
							mutationsEnabled={MUTATIONS_ENABLED}
							onCleared={() => {
								faultRefreshNonce += 1;
								liveFaults = liveFaults.filter((fault) => fault.component !== selectedEcu);
							}}
						/>
						<UC04Pagination
							total={filteredCount}
							pageSize={PAGE_SIZE}
							page={dtcPage}
							onPage={(pageNumber) => (dtcPage = pageNumber)}
						/>
					</div>
				</section>

				<UC05FaultsTimeline extraFaults={liveFaults} refreshNonce={faultRefreshNonce} />
			</div>

			<!-- Selected component -->
			<div class="flex flex-col gap-6">
				<section class="rounded-md border border-border bg-card p-4">
					<h3 class="mb-3 text-sm font-semibold">
						Component — <span class="font-mono text-muted-foreground">{selectedEcu}</span>
					</h3>
					<UC09HwSwVersion componentId={selectedEcu} />
					<div class="my-3 border-t border-border"></div>
					<UC10LiveDidReads componentId={selectedEcu} />
				</section>

				<UC06Operations componentId={selectedEcu} controlEnabled={MUTATIONS_ENABLED} />
			</div>

			<!-- System -->
			<div class="flex flex-col gap-6">
				<UC18GatewayRouting />
				<UC15Session />
				<UC16AuditLog />
			</div>
		</div>

		<SystemTopology />

		<section class="space-y-2">
			<div class="flex items-center justify-between gap-3">
				<h2 class="text-sm font-semibold">Historical trends</h2>
				<button
					onclick={() => (showHistorical = !showHistorical)}
					class="rounded border border-border bg-card px-3 py-1.5 text-xs font-medium hover:bg-muted"
				>
					{showHistorical ? 'Hide panel' : 'Show panel'}
				</button>
			</div>
			<UC19Historical visible={showHistorical} grafanaUrl={import.meta.env.VITE_GRAFANA_URL ?? ''} />
		</section>
	</main>
</div>
