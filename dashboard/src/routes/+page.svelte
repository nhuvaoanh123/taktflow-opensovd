<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Taktflow OpenSOVD - Live SIL Operations Dashboard -->
<script lang="ts">
	import { onMount } from 'svelte';

	import { subscribe } from '$lib/api/wsClient';
	import type { DtcEntry, EcuId, TelemetryFrame } from '$lib/types/sovd';

	import UC01DtcList from '$lib/widgets/UC01DtcList.svelte';
	import UC02DtcDetail from '$lib/widgets/UC02DtcDetail.svelte';
	import UC03ClearFaults from '$lib/widgets/UC03ClearFaults.svelte';
	import UC04Pagination from '$lib/widgets/UC04Pagination.svelte';
	import UC05FaultsTimeline from '$lib/widgets/UC05FaultsTimeline.svelte';
	import UC06Operations from '$lib/widgets/UC06Operations.svelte';
	import UC07RoutineCatalog from '$lib/widgets/UC07RoutineCatalog.svelte';
	import UC08ComponentCards from '$lib/widgets/UC08ComponentCards.svelte';
	import UC09HwSwVersion from '$lib/widgets/UC09HwSwVersion.svelte';
	import UC10LiveDidReads from '$lib/widgets/UC10LiveDidReads.svelte';
	import UC11FaultPipeline from '$lib/widgets/UC11FaultPipeline.svelte';
	import UC12OperationCycle from '$lib/widgets/UC12OperationCycle.svelte';
	import UC14CdaTopology from '$lib/widgets/UC14CdaTopology.svelte';
	import UC15Session from '$lib/widgets/UC15Session.svelte';
	import UC16AuditLog from '$lib/widgets/UC16AuditLog.svelte';
	import UC18GatewayRouting from '$lib/widgets/UC18GatewayRouting.svelte';
	import UC19Historical from '$lib/widgets/UC19Historical.svelte';

	let selectedEcu = $state<EcuId>('cvc');
	let selectedDtc = $state<DtcEntry | null>(null);
	let dtcPage = $state(0);
	let showHistorical = $state(false);
	let filteredCount = $state(0);
	let faultRefreshNonce = $state(0);
	let liveFaults = $state<DtcEntry[]>([]);

	const PAGE_SIZE = 5;
	const MUTATIONS_ENABLED = import.meta.env.VITE_SIL_MUTATIONS_ENABLED === 'true';

	onMount(() => {
		return subscribe((frame: TelemetryFrame) => {
			if (frame.type !== 'dtc') {
				return;
			}
			liveFaults = [frame.payload as DtcEntry, ...liveFaults].slice(0, 50);
		});
	});
</script>

<svelte:head>
	<title>Taktflow OpenSOVD - Live SIL</title>
</svelte:head>

<UC02DtcDetail dtc={selectedDtc} onClose={() => (selectedDtc = null)} />

<div class="min-h-screen bg-background text-foreground">
	<header class="border-b border-border bg-card">
		<div class="mx-auto flex max-w-[1800px] flex-wrap items-center justify-between gap-4 px-6 py-3">
			<div>
				<h1 class="text-lg font-semibold tracking-tight">OpenSOVD SIL Operations</h1>
				<p class="text-xs text-muted-foreground">
					Public simulator environment: sovd-main, CDA, ECU simulator, MQTT
				</p>
			</div>
			<nav class="flex flex-wrap items-center gap-2 text-xs">
				<a
					href="https://taktflow-systems.com/"
					class="rounded border border-border bg-white px-3 py-1.5 font-medium text-foreground hover:bg-muted"
				>
					Taktflow Systems
				</a>
				<a
					href="/sovd/"
					class="rounded border border-border bg-white px-3 py-1.5 font-medium text-foreground hover:bg-muted"
				>
					Engineering spec
				</a>
				<a
					href="/sovd/grafana/"
					class="rounded border border-border bg-white px-3 py-1.5 font-medium text-foreground hover:bg-muted"
				>
					Grafana
				</a>
			</nav>
			<div class="flex items-center gap-2 text-xs text-muted-foreground">
				<span class="h-2 w-2 rounded-full bg-emerald-600"></span>
				<span>Public endpoint online</span>
			</div>
		</div>
	</header>

	<main class="mx-auto flex max-w-[1800px] flex-col gap-4 px-6 py-4">
		<section class="space-y-2">
			<div class="flex flex-wrap items-end justify-between gap-2">
				<div>
					<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
						Component inventory
					</h2>
					<p class="text-xs text-muted-foreground">
						Selected component: <span class="font-mono text-foreground">{selectedEcu}</span>
					</p>
				</div>
				<span class="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground">
					{MUTATIONS_ENABLED ? 'operator controls enabled' : 'public read-only mode'}
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

		<div class="grid flex-1 gap-4 xl:grid-cols-[minmax(0,1.05fr)_minmax(0,1fr)_minmax(0,1fr)]">
			<div class="flex flex-col gap-4">
				<section class="rounded-md border border-border bg-card p-3">
					<div class="mb-3 flex items-center justify-between gap-2">
						<h2 class="text-sm font-semibold">Faults</h2>
						<span class="font-mono text-xs text-muted-foreground">{selectedEcu}</span>
					</div>
					<UC01DtcList
						componentId={selectedEcu}
						page={dtcPage}
						pageSize={PAGE_SIZE}
						refreshNonce={faultRefreshNonce}
						onSelect={(dtc) => (selectedDtc = dtc)}
						onPage={(pageNumber) => (dtcPage = pageNumber)}
						onTotalChange={(total) => (filteredCount = total)}
					/>
					<div class="mt-3 flex flex-wrap items-center justify-between gap-2">
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
				<UC16AuditLog />
			</div>

			<div class="flex flex-col gap-4">
				<UC06Operations componentId={selectedEcu} controlEnabled={MUTATIONS_ENABLED} />
				<UC07RoutineCatalog componentId={selectedEcu} />
				<UC09HwSwVersion componentId={selectedEcu} />
				<UC10LiveDidReads componentId={selectedEcu} />
			</div>

			<div class="flex flex-col gap-4">
				<UC18GatewayRouting />
				<UC14CdaTopology />
				<UC11FaultPipeline />
				<UC12OperationCycle />
				<UC15Session />
			</div>
		</div>

		<section class="space-y-2">
			<div class="flex items-center justify-between gap-3">
				<h2 class="text-sm font-semibold">Historical trends</h2>
				<button
					onclick={() => (showHistorical = !showHistorical)}
					class="rounded border border-border bg-white px-3 py-1.5 text-xs font-medium hover:bg-muted"
				>
					{showHistorical ? 'Hide panel' : 'Show panel'}
				</button>
			</div>
			<UC19Historical visible={showHistorical} grafanaUrl={import.meta.env.VITE_GRAFANA_URL ?? ''} />
		</section>
	</main>
</div>
