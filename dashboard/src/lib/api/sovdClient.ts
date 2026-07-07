// SPDX-License-Identifier: Apache-2.0
// Typed fetch wrappers for the OpenSOVD REST API.
// Every function returns live backend data, or `null` when the route is
// unavailable so widgets can render an explicit unavailable state. Canned
// data is never substituted for a live route; the remaining canned
// constants exist only for unmounted showcase widgets and the explicitly
// labelled UC21 stub path.

import type {
	AuditEntry,
	ComponentSource,
	DtcEntry,
	DtcStatus,
	EcuId,
	GatewayBackend,
	GatewayHealth,
	GatewayHealthProbe,
	LiveDid,
	MlInferenceResult,
	RoutineEntry,
	RoutineStatus,
	SessionInfo,
	SovdComponent
} from '$lib/types/sovd';

type DiscoveredEntitiesResponse = {
	items?: Array<{
		id?: string;
		name?: string;
		href?: string;
	}>;
};

type EntityCapabilitiesResponse = {
	id?: string;
	name?: string;
	data?: string | null;
	faults?: string | null;
	operations?: string | null;
	modes?: string | null;
	variant?: {
		logical_address?: string | null;
		name?: string | null;
		state?: string | null;
		is_base_variant?: boolean | null;
	} | null;
};

type FaultResponse = {
	code?: string;
	display_code?: string | null;
	fault_name?: string;
	severity?: number | string | null;
	status?: Record<string, unknown> | string | null;
};

type ListOfFaultsResponse = {
	items?: FaultResponse[];
};

type FaultDetailsResponse = {
	item?: FaultResponse;
	environment_data?: unknown;
};

type OperationResponse = {
	id?: string;
	name?: string | null;
};

type OperationsListResponse = {
	items?: OperationResponse[];
};

type StartExecutionAsyncResponse = {
	id?: string;
	status?: string | null;
};

type ExecutionStatusResponse = {
	id?: string | null;
	status?: string | null;
	result?: Record<string, unknown> | null;
	parameters?: Record<string, unknown> | null;
	error?: Array<{
		title?: string;
		message?: string;
	}> | null;
};

type DataMetadataResponse = {
	id?: string;
	name?: string | null;
	category?: string | null;
};

type DatasResponse = {
	items?: DataMetadataResponse[];
};

type ReadValueResponse = {
	id?: string;
	data?: unknown;
};

type MlInferenceStartResponse = {
	id?: string | null;
	status?: string | null;
	result?: Record<string, unknown> | null;
};

type BackendHealthResponse = {
	status?: string | null;
	reason?: string | null;
};

type GatewayHealthResponse = {
	status?: string | null;
	version?: string | null;
	sovd_db?: BackendHealthResponse | null;
	fault_sink?: BackendHealthResponse | null;
	operation_cycle?: string | null;
};

type SessionResponse = {
	session_id?: string | null;
	level?: string | null;
	security_level?: number | null;
	expires_at_ms?: number | null;
	active?: boolean | null;
};

type AuditLogResponse = {
	items?: Array<{
		timestamp_ms?: number | null;
		actor?: string | null;
		action?: string | null;
		target?: string | null;
		result?: string | null;
	}>;
};

type BackendRoutesResponse = {
	items?: Array<{
		id?: string | null;
		address?: string | null;
		protocol?: string | null;
		reachable?: boolean | null;
		latency_ms?: number | null;
	}>;
};

const DEFAULT_BASE = 'http://localhost:21002';
const EXECUTION_IDS = new Map<string, string>();

// Components served by the local sovd-server simulators (as opposed to
// CDA-forwarded entities); used only for source classification.
const LOCAL_COMPONENT_IDS: readonly EcuId[] = ['cvc', 'sc', 'bcm'];

function apiBase(): string {
	if (typeof window === 'undefined') {
		return DEFAULT_BASE;
	}
	const configured = import.meta.env.VITE_SOVD_BASE;
	if (typeof configured === 'string') {
		return configured;
	}
	return import.meta.env.PROD ? '' : DEFAULT_BASE;
}

function sessionValue(key: string): string | null {
	if (typeof window === 'undefined') {
		return null;
	}
	try {
		return window.sessionStorage.getItem(key);
	} catch {
		return null;
	}
}

function bearerToken(): string {
	if (typeof window === 'undefined') {
		return '';
	}
	return import.meta.env.VITE_SOVD_TOKEN ?? sessionValue('sovdBearerToken') ?? '';
}

function requestHeaders(extra?: HeadersInit): HeadersInit {
	const token = bearerToken();
	return {
		Accept: 'application/json',
		...(token ? { Authorization: `Bearer ${token}` } : {}),
		...extra
	};
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
	const response = await fetch(`${apiBase()}${path}`, {
		...init,
		headers: requestHeaders(init?.headers)
	});
	if (!response.ok) {
		throw new Error(`${init?.method ?? 'GET'} ${path} failed: ${response.status}`);
	}
	if (response.status === 204) {
		return undefined as T;
	}
	return (await response.json()) as T;
}

function nowMs(): number {
	if (typeof performance !== 'undefined') {
		return performance.now();
	}
	return Date.now();
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return !!value && typeof value === 'object' && !Array.isArray(value);
}

function isComponentId(value: unknown): value is EcuId {
	return typeof value === 'string' && value.trim().length > 0;
}

function capabilitiesFromEntity(entity?: EntityCapabilitiesResponse): SovdComponent['capabilities'] {
	const capabilities: SovdComponent['capabilities'] = [];
	if (entity?.faults) capabilities.push('faults');
	if (entity?.operations) capabilities.push('operations');
	if (entity?.data) capabilities.push('data');
	if (entity?.modes) capabilities.push('modes');
	return capabilities;
}

function mapComponent(
	id: EcuId,
	name: string | undefined,
	entity?: EntityCapabilitiesResponse
): SovdComponent {
	return {
		id,
		label: name?.trim() || entity?.name?.trim() || id.toUpperCase(),
		capabilities: capabilitiesFromEntity(entity),
		source: componentSource(id, entity),
		logicalAddress: entity?.variant?.logical_address ?? undefined,
		state: entity?.variant?.state ?? undefined
	};
}

function componentSource(
	id: EcuId,
	entity?: EntityCapabilitiesResponse
): ComponentSource {
	if (id === 'dfm') {
		return 'dfm';
	}
	if (entity?.variant || entity?.faults?.includes('/vehicle/v15/')) {
		return 'cda';
	}
	if (LOCAL_COMPONENT_IDS.includes(id)) {
		return 'local';
	}
	return 'unknown';
}

function severityFromValue(value: number | string | null | undefined): DtcEntry['severity'] {
	if (typeof value === 'string') {
		const normalized = value.toLowerCase();
		if (normalized === 'critical' || normalized === 'high' || normalized === 'medium') {
			return normalized;
		}
		return 'low';
	}
	switch (value) {
		case 1:
			return 'critical';
		case 2:
			return 'high';
		case 3:
			return 'medium';
		default:
			return 'low';
	}
}

function truthy(value: unknown): boolean {
	if (value === true || value === 1 || value === '1') {
		return true;
	}
	if (typeof value !== 'string') {
		return false;
	}
	const normalized = value.trim().toLowerCase();
	return normalized === 'true' || normalized === 'yes' || normalized === 'active';
}

function statusFromValue(value: unknown): DtcStatus {
	if (typeof value === 'string') {
		const normalized = value.trim().toLowerCase().replaceAll('-', '_');
		if (normalized === 'confirmed') return 'confirmed';
		if (normalized === 'pending') return 'pending';
		if (normalized === 'cleared' || normalized === 'inactive') return 'cleared';
		if (normalized === 'suppressed') return 'suppressed';
		if (normalized === 'test_failed' || normalized === 'active') return 'test_failed';
		return 'warning_indicator';
	}
	if (!value || typeof value !== 'object') {
		return 'warning_indicator';
	}
	const status = value as Record<string, unknown>;
	const aggregatedStatus = status.aggregatedStatus ?? status.aggregated_status;
	if (truthy(status.confirmedDTC ?? status.confirmed_dtc)) return 'confirmed';
	if (truthy(status.pendingDTC ?? status.pending_dtc) || aggregatedStatus === 'pending') {
		return 'pending';
	}
	if (truthy(status.suppressedDTC ?? status.suppressed_dtc) || aggregatedStatus === 'suppressed') {
		return 'suppressed';
	}
	if (aggregatedStatus === 'cleared' || aggregatedStatus === 'inactive') {
		return 'cleared';
	}
	if (truthy(status.testFailed ?? status.test_failed) || aggregatedStatus === 'active') {
		return 'test_failed';
	}
	return 'warning_indicator';
}

function routineStatusFromExecution(value: string | null | undefined): RoutineStatus {
	const normalized = value?.toLowerCase();
	if (normalized === 'running') return 'running';
	if (normalized === 'completed') return 'completed';
	if (normalized === 'failed') return 'failed';
	return 'idle';
}

function freezeFrameFromEnvironment(environment: unknown): Record<string, string> | undefined {
	if (!environment || typeof environment !== 'object') {
		return undefined;
	}
	const source =
		'data' in (environment as Record<string, unknown>) &&
		(environment as Record<string, unknown>).data &&
		typeof (environment as Record<string, unknown>).data === 'object'
			? ((environment as Record<string, unknown>).data as Record<string, unknown>)
			: (environment as Record<string, unknown>);
	const entries = Object.entries(source)
		.filter(([, value]) => value !== null && value !== undefined)
		.map(([key, value]) => [key, typeof value === 'string' ? value : JSON.stringify(value)] as const);
	return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function mapFault(componentId: EcuId, fault: FaultResponse, detail?: FaultDetailsResponse): DtcEntry {
	const code = fault.display_code ?? fault.code ?? 'UNKNOWN';
	// fault_name is an ODX short-name (identifier, underscores instead of
	// spaces); render it as prose without altering the underlying record.
	const description = fault.fault_name?.replaceAll('_', ' ') ?? 'Reported fault';
	return {
		id: `${componentId}:${fault.code ?? code}`,
		code,
		description,
		severity: severityFromValue(fault.severity),
		status: statusFromValue(fault.status),
		component: componentId,
		freezeFrame: freezeFrameFromEnvironment(detail?.environment_data)
	};
}

function executionKey(componentId: EcuId, routineId: string): string {
	return `${componentId}:${routineId}`;
}

function plainRoutine(componentId: EcuId, routineId: string): RoutineEntry {
	return {
		id: routineId,
		name: routineId,
		component: componentId,
		status: 'idle'
	};
}

function findDataId(
	items: DataMetadataResponse[],
	patterns: readonly RegExp[]
): string | undefined {
	return items.find((item) => {
		const haystack = [item.id, item.name, item.category]
			.filter((value): value is string => typeof value === 'string')
			.join(' ')
			.toLowerCase();
		return patterns.some((pattern) => pattern.test(haystack));
	})?.id;
}

function extractStringValue(value: unknown): string | undefined {
	if (typeof value === 'string' && value.trim()) {
		return value.trim();
	}
	if (!isRecord(value)) {
		return undefined;
	}
	for (const key of ['value', 'current', 'raw', 'vin']) {
		const nested = extractStringValue(value[key]);
		if (nested) {
			return nested;
		}
	}
	return undefined;
}

function extractNumberValue(value: unknown): number | undefined {
	if (typeof value === 'number' && Number.isFinite(value)) {
		return value;
	}
	if (typeof value === 'string') {
		const parsed = Number.parseFloat(value);
		return Number.isFinite(parsed) ? parsed : undefined;
	}
	if (!isRecord(value)) {
		return undefined;
	}
	for (const key of ['value', 'current', 'raw', 'reading']) {
		const nested = extractNumberValue(value[key]);
		if (nested !== undefined) {
			return nested;
		}
	}
	return undefined;
}

function extractBooleanValue(value: unknown): boolean | undefined {
	if (typeof value === 'boolean') {
		return value;
	}
	if (typeof value === 'number') {
		return value !== 0;
	}
	if (typeof value === 'string') {
		const normalized = value.trim().toLowerCase();
		if (normalized === 'true' || normalized === 'yes' || normalized === '1') {
			return true;
		}
		if (normalized === 'false' || normalized === 'no' || normalized === '0') {
			return false;
		}
	}
	if (!isRecord(value)) {
		return undefined;
	}
	for (const key of ['value', 'active', 'current']) {
		const nested = extractBooleanValue(value[key]);
		if (nested !== undefined) {
			return nested;
		}
	}
	return undefined;
}

function mapGatewayProbe(probe?: BackendHealthResponse | null): GatewayHealthProbe {
	const status = probe?.status?.toLowerCase();
	if (status === 'degraded' || status === 'unavailable') {
		return {
			status,
			reason: probe?.reason ?? undefined
		};
	}
	return {
		status: 'ok',
		reason: probe?.reason ?? undefined
	};
}

function clampSecurityLevel(value: number | null | undefined): 0 | 1 | 2 | 3 {
	if (value === 1 || value === 2 || value === 3) {
		return value;
	}
	return 0;
}

function mapSessionLevel(value: string | null | undefined): SessionInfo['level'] {
	switch ((value ?? '').toLowerCase()) {
		case 'programming':
			return 'programming';
		case 'extended':
			return 'extended';
		default:
			return 'default';
	}
}

function timestampFromMs(value: number | null | undefined): string {
	if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
		return new Date(value).toISOString();
	}
	return new Date().toISOString();
}

function mapAuditResult(value: string | null | undefined): AuditEntry['result'] {
	switch ((value ?? '').toLowerCase()) {
		case 'denied':
			return 'denied';
		case 'error':
			return 'error';
		default:
			return 'ok';
	}
}

function mapGatewayProtocol(value: string | null | undefined): GatewayBackend['protocol'] {
	switch ((value ?? '').toLowerCase()) {
		case 'uds':
			return 'uds';
		case 'doip':
			return 'doip';
		default:
			return 'sovd';
	}
}

function inferencePredictionFromValue(value: unknown): MlInferenceResult['prediction'] {
	switch (String(value ?? '').toLowerCase()) {
		case 'warning':
			return 'warning';
		case 'critical':
			return 'critical';
		default:
			return 'normal';
	}
}

function mlInferencePath(componentId: EcuId): string {
	return `/sovd/v1/components/${componentId}/operations/ml-inference/executions`;
}

function mapMlInferenceResult(
	componentId: EcuId,
	result: Record<string, unknown> | null | undefined
): MlInferenceResult {
	const fallback = cannedMlInference(componentId);
	const confidence =
		typeof result?.confidence === 'number' && Number.isFinite(result.confidence)
			? result.confidence
			: fallback.confidence;
	const source = extractStringValue(result?.source)?.toLowerCase();
	const status = extractStringValue(result?.status)?.toLowerCase();
	const rollback = isRecord(result?.rollback) ? result.rollback : undefined;
	const lifecycleState = extractStringValue(result?.lifecycle_state)?.toLowerCase();

	return {
		component: componentId,
		modelName: extractStringValue(result?.model_name) ?? fallback.modelName,
		modelVersion: extractStringValue(result?.model_version) ?? fallback.modelVersion,
		prediction: inferencePredictionFromValue(result?.prediction ?? fallback.prediction),
		confidence,
		fingerprint: extractStringValue(result?.fingerprint) ?? fallback.fingerprint,
		updatedAt: extractStringValue(result?.updated_at) ?? fallback.updatedAt,
		source: source === 'live' ? 'live' : fallback.source,
		status:
			status === 'running' || status === 'failed' || status === 'completed'
				? status
				: fallback.status,
		advisoryOnly: extractBooleanValue(result?.advisory_only) ?? fallback.advisoryOnly,
		advisoryActive: extractBooleanValue(result?.advisory_active) ?? fallback.advisoryActive,
		lifecycleState: lifecycleState === 'rolled_back' ? 'rolled_back' : fallback.lifecycleState,
		rollbackTrigger: extractStringValue(rollback?.trigger) ?? fallback.rollbackTrigger,
		rollbackAt: extractStringValue(rollback?.at) ?? fallback.rollbackAt,
		rollbackFromModelVersion:
			extractStringValue(rollback?.from_model_version) ?? fallback.rollbackFromModelVersion,
		rollbackToModelVersion:
			extractStringValue(rollback?.to_model_version) ?? fallback.rollbackToModelVersion
	};
}

export async function listComponents(): Promise<SovdComponent[] | null> {
	try {
		const discovered = await fetchJson<DiscoveredEntitiesResponse>('/sovd/v1/components');
		const items = discovered.items ?? [];
		const liveItems = items.filter(
			(item): item is { id: EcuId; name?: string } => isComponentId(item.id)
		);
		const capabilities = await Promise.all(
			liveItems.map(async (item) => {
				try {
					return await fetchJson<EntityCapabilitiesResponse>(`/sovd/v1/components/${item.id}`);
				} catch {
					return undefined;
				}
			})
		);
		return liveItems.map((item, index) => mapComponent(item.id, item.name, capabilities[index]));
	} catch {
		return null;
	}
}

export async function getComponent(id: EcuId): Promise<SovdComponent | null> {
	try {
		const entity = await fetchJson<EntityCapabilitiesResponse>(`/sovd/v1/components/${id}`);
		return mapComponent(id, entity.name, entity);
	} catch {
		return null;
	}
}

export async function listFaults(
	componentId: EcuId,
	statusMask?: DtcStatus
): Promise<DtcEntry[] | null> {
	try {
		const response = await fetchJson<ListOfFaultsResponse>(
			`/sovd/v1/components/${componentId}/faults?page=1&page-size=200`
		);
		const faults = (response.items ?? []).map((fault) => mapFault(componentId, fault));
		return statusMask ? faults.filter((fault) => fault.status === statusMask) : faults;
	} catch {
		return null;
	}
}

function faultTimeMs(value: string | undefined): number {
	const parsed = value ? Date.parse(value) : Number.NaN;
	return Number.isFinite(parsed) ? parsed : 0;
}

// Deterministic fault order: last-seen time (newest first) where the ECU
// reports one, then code, then component. The fault routes return items in
// arbitrary order, so an explicit tiebreak keeps rows stable across loads.
export function compareFaults(left: DtcEntry, right: DtcEntry): number {
	const timeDelta = faultTimeMs(right.lastSeen) - faultTimeMs(left.lastSeen);
	if (timeDelta !== 0) {
		return timeDelta;
	}
	return left.code.localeCompare(right.code) || left.component.localeCompare(right.component);
}

export async function listAllFaults(componentIds?: readonly EcuId[]): Promise<DtcEntry[] | null> {
	const ids = componentIds ?? (await listComponents())?.map((component) => component.id);
	if (!ids) {
		return null;
	}
	const batches = await Promise.all(ids.map((componentId) => listFaults(componentId)));
	const reachable = batches.filter((batch): batch is DtcEntry[] => batch !== null);
	if (ids.length > 0 && reachable.length === 0) {
		return null;
	}
	return reachable.flat().sort(compareFaults);
}

export async function getFaultDetail(componentId: EcuId, dtcId: string): Promise<DtcEntry | null> {
	const code = dtcId.includes(':') ? dtcId.split(':').pop() ?? dtcId : dtcId;
	try {
		const detail = await fetchJson<FaultDetailsResponse>(
			`/sovd/v1/components/${componentId}/faults/${encodeURIComponent(code)}`
		);
		return mapFault(componentId, detail.item ?? { code }, detail);
	} catch {
		return null;
	}
}

export async function clearFaults(componentId: EcuId, group?: string): Promise<void> {
	const path = group
		? `/sovd/v1/components/${componentId}/faults/${encodeURIComponent(group)}`
		: `/sovd/v1/components/${componentId}/faults`;
	await fetchJson<void>(path, { method: 'DELETE' });
}

export async function listRoutines(componentId: EcuId): Promise<RoutineEntry[] | null> {
	try {
		const response = await fetchJson<OperationsListResponse>(
			`/sovd/v1/components/${componentId}/operations`
		);
		return (response.items ?? []).map((operation) => {
			const base = plainRoutine(componentId, operation.id ?? 'unknown-operation');
			return {
				...base,
				name: operation.name?.trim() || base.name
			};
		});
	} catch {
		return null;
	}
}

export async function startRoutine(componentId: EcuId, routineId: string): Promise<void> {
	const response = await fetchJson<StartExecutionAsyncResponse>(
		`/sovd/v1/components/${componentId}/operations/${encodeURIComponent(routineId)}/executions`,
		{
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({})
		}
	);
	if (response?.id) {
		EXECUTION_IDS.set(executionKey(componentId, routineId), response.id);
	}
}

export async function stopRoutine(componentId: EcuId, routineId: string): Promise<void> {
	EXECUTION_IDS.delete(executionKey(componentId, routineId));
}

export async function pollRoutine(
	componentId: EcuId,
	routineId: string
): Promise<RoutineEntry | null> {
	const key = executionKey(componentId, routineId);
	const executionId = EXECUTION_IDS.get(key);
	if (!executionId) {
		return plainRoutine(componentId, routineId);
	}
	try {
		const response = await fetchJson<ExecutionStatusResponse>(
			`/sovd/v1/components/${componentId}/operations/${encodeURIComponent(routineId)}/executions/${encodeURIComponent(executionId)}`
		);
		return {
			...plainRoutine(componentId, routineId),
			status: routineStatusFromExecution(response.status),
			lastResult:
				response.error && response.error.length > 0
					? response.error.map((entry) => entry.message ?? entry.title ?? 'error').join('; ')
					: undefined
		};
	} catch {
		return null;
	}
}

export async function readDid(componentId: EcuId): Promise<LiveDid | null> {
	try {
		const listing = await fetchJson<DatasResponse>(`/sovd/v1/components/${componentId}/data`);
		const items = (listing.items ?? []).filter(
			(item): item is DataMetadataResponse & { id: string } => typeof item.id === 'string'
		);
		const vinId = findDataId(items, [/^vin$/i, /\bvin\b/i, /vehicle identification/i]);
		const batteryId = findDataId(items, [
			/^battery_voltage$/i,
			/\bbattery[_ -]?voltage\b/i,
			/\bvoltage\b/i
		]);
		const temperatureId = findDataId(items, [/^temperature$/i, /\btemp(?:erature)?\b/i]);
		const [vinValue, batteryValue, temperatureValue] = await Promise.all([
			vinId
				? fetchJson<ReadValueResponse>(
						`/sovd/v1/components/${componentId}/data/${encodeURIComponent(vinId)}`
					).catch(() => undefined)
				: Promise.resolve(undefined),
			batteryId
				? fetchJson<ReadValueResponse>(
						`/sovd/v1/components/${componentId}/data/${encodeURIComponent(batteryId)}`
					).catch(() => undefined)
				: Promise.resolve(undefined),
			temperatureId
				? fetchJson<ReadValueResponse>(
						`/sovd/v1/components/${componentId}/data/${encodeURIComponent(temperatureId)}`
					).catch(() => undefined)
				: Promise.resolve(undefined)
		]);
		return {
			component: componentId,
			vin: extractStringValue(vinValue?.data),
			batteryVoltage: extractNumberValue(batteryValue?.data),
			temperature: extractNumberValue(temperatureValue?.data),
			timestamp: new Date().toISOString()
		};
	} catch {
		return null;
	}
}

export async function runMlInference(
	componentId: EcuId,
	parameters: Record<string, unknown> = {}
): Promise<MlInferenceResult> {
	try {
		const start = await fetchJson<MlInferenceStartResponse>(mlInferencePath(componentId), {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				parameters: {
					mode: 'single-shot',
					component_id: componentId,
					...parameters
				}
			})
		});

		if (isRecord(start.result)) {
			return mapMlInferenceResult(componentId, {
				...start.result,
				status: start.status ?? start.result.status ?? 'completed',
				source: 'live'
			});
		}

		if (start.id) {
			const polled = await fetchJson<ExecutionStatusResponse>(
				`${mlInferencePath(componentId)}/${encodeURIComponent(start.id)}`
			);
			return mapMlInferenceResult(componentId, {
				...(isRecord(polled.parameters)
					? polled.parameters
					: isRecord(polled.result)
						? polled.result
						: {}),
				status: polled.status ?? 'completed',
				source: 'live'
			});
		}
	} catch {
		// Demo-safe fallback when the future ML route is not mounted yet.
	}

	return cannedMlInference(componentId);
}

export async function getSession(): Promise<SessionInfo | null> {
	try {
		const response = await fetchJson<SessionResponse>('/sovd/v1/session');
		return {
			sessionId: response.session_id?.trim() || '--',
			level: mapSessionLevel(response.level),
			securityLevel: clampSecurityLevel(response.security_level),
			expiresAt:
				typeof response.expires_at_ms === 'number'
					? new Date(response.expires_at_ms).toISOString()
					: undefined,
			active: response.active ?? true
		};
	} catch {
		return null;
	}
}

export async function getGatewayHealth(): Promise<GatewayHealth | null> {
	const startedAt = nowMs();
	try {
		const response = await fetchJson<GatewayHealthResponse>('/sovd/v1/health');
		return {
			status: response.status ?? 'ok',
			version: response.version ?? '--',
			sovdDb: mapGatewayProbe(response.sovd_db),
			faultSink: mapGatewayProbe(response.fault_sink),
			operationCycle: response.operation_cycle ?? undefined,
			latencyMs: Math.max(1, Math.round(nowMs() - startedAt))
		};
	} catch {
		return null;
	}
}

export async function listGatewayBackends(): Promise<GatewayBackend[] | null> {
	try {
		const response = await fetchJson<BackendRoutesResponse>('/sovd/v1/gateway/backends');
		const items = response.items ?? [];
		return items
			.map((item) => {
				const id = item.id?.trim();
				if (!id) {
					return null;
				}
				return {
					id,
					address: item.address?.trim() || '--',
					protocol: mapGatewayProtocol(item.protocol),
					reachable: item.reachable !== false,
					latencyMs:
						typeof item.latency_ms === 'number' && Number.isFinite(item.latency_ms)
							? Math.max(0, Math.round(item.latency_ms))
							: 0
				} satisfies GatewayBackend;
			})
			.filter((item): item is GatewayBackend => item !== null);
	} catch {
		return null;
	}
}

export async function getAuditLog(limit = 50): Promise<AuditEntry[] | null> {
	try {
		const response = await fetchJson<AuditLogResponse>(`/sovd/v1/audit?limit=${limit}`);
		const items = response.items ?? [];
		return items
			.map((item) => ({
				timestamp: timestampFromMs(item.timestamp_ms),
				actor: item.actor?.trim() || 'observer',
				action: item.action?.trim() || 'UNKNOWN',
				target: item.target?.trim() || '--',
				result: mapAuditResult(item.result)
			}))
			.slice(0, limit);
	} catch {
		return null;
	}
}

export function telemetryPayloadToDtc(payload: unknown): DtcEntry | null {
	if (!payload || typeof payload !== 'object') {
		return null;
	}
	const record = payload as Record<string, unknown>;
	const componentId = record.component_id ?? record.componentId ?? record.component;
	const code = record.dtc ?? record.code;
	if (!isComponentId(componentId) || typeof code !== 'string') {
		return null;
	}
	const id = componentId.trim();
	const timestamp =
		typeof record.timestamp === 'string' ? record.timestamp : new Date().toISOString();
	return {
		id: `${id}:${code}:${timestamp}`,
		code,
		description:
			typeof record.description === 'string'
				? record.description
				: 'Fault event relayed via MQTT bridge',
		severity: severityFromValue(record.severity as number | string | null | undefined),
		status: statusFromValue(record.status),
		firstSeen: timestamp,
		lastSeen: timestamp,
		occurrences: 1,
		component: id,
		ecuAddress: 0
	};
}

// Illustrative DTC set for the unmounted UC13 lifecycle showcase widget.
// Never substituted for a live route.
export const CANNED_DTCS: DtcEntry[] = [
	{
		id: 'dtc-001',
		code: 'P0A1F',
		description: 'High Voltage Battery Pack Voltage Sense Circuit',
		severity: 'critical',
		status: 'confirmed',
		component: 'cvc',
		ecuAddress: 0x7e0,
		firstSeen: '2026-04-15T08:22:11Z',
		lastSeen: '2026-04-17T09:44:02Z',
		occurrences: 7,
		freezeFrame: { 'Battery Voltage': '3.2V', Temperature: '45C', SOC: '12%' }
	},
	{
		id: 'dtc-002',
		code: 'C0035',
		description: 'Left Front Wheel Speed Sensor Circuit',
		severity: 'medium',
		status: 'pending',
		component: 'cvc',
		ecuAddress: 0x7e0,
		firstSeen: '2026-04-16T14:10:00Z',
		lastSeen: '2026-04-17T09:44:02Z',
		occurrences: 3
	},
	{
		id: 'dtc-003',
		code: 'U0100',
		description: 'Lost Communication With ECM/PCM A',
		severity: 'high',
		status: 'confirmed',
		component: 'sc',
		ecuAddress: 0x7e4,
		firstSeen: '2026-04-14T12:00:00Z',
		lastSeen: '2026-04-17T07:30:00Z',
		occurrences: 12
	},
	{
		id: 'dtc-004',
		code: 'B1234',
		description: 'Driver Door Ajar Switch Circuit Open',
		severity: 'low',
		status: 'cleared',
		component: 'bcm',
		ecuAddress: 0x726,
		firstSeen: '2026-04-10T10:00:00Z',
		lastSeen: '2026-04-11T11:00:00Z',
		occurrences: 1
	},
	{
		id: 'dtc-005',
		code: 'P0D00',
		description: 'Electric Vehicle Battery Pack Malfunction',
		severity: 'critical',
		status: 'test_failed',
		component: 'cvc',
		ecuAddress: 0x7e0,
		firstSeen: '2026-04-17T08:00:00Z',
		lastSeen: '2026-04-17T09:44:02Z',
		occurrences: 2
	},
	{
		id: 'dtc-006',
		code: 'C0051',
		description: 'Steering Position Sensor Circuit',
		severity: 'medium',
		status: 'suppressed',
		component: 'sc',
		ecuAddress: 0x7e4,
		firstSeen: '2026-04-12T09:15:00Z',
		lastSeen: '2026-04-12T09:15:00Z',
		occurrences: 1
	}
];

export function cannedMlInference(componentId: EcuId): MlInferenceResult {
	return {
		component: componentId,
		modelName: 'reference-fault-predictor',
		modelVersion: '1.0.0-rc1',
		prediction: componentId === 'cvc' ? 'warning' : 'normal',
		confidence: componentId === 'cvc' ? 0.82 : 0.94,
		fingerprint: 'sha256:7b0f1b5f2b8c2a7e8d4d0f9c3f6b1a22',
		updatedAt: new Date().toISOString(),
		source: 'stub',
		status: 'completed',
		advisoryOnly: true,
		advisoryActive: componentId === 'cvc',
		lifecycleState: 'ready'
	};
}
