// SPDX-License-Identifier: Apache-2.0
// Typed fetch wrappers for the OpenSOVD REST API.
// Where the Phase 5 backend already exposes a live endpoint, we use it.
// For slices that are still scaffold-only, we fall back to canned data so
// the Stage 1 dashboard stays usable on a partially wired bench.

import type {
	AuditEntry,
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

export const BENCH_COMPONENT_IDS: readonly EcuId[] = ['cvc', 'sc', 'bcm'];

function apiBase(): string {
	if (typeof window === 'undefined') {
		return DEFAULT_BASE;
	}
	return import.meta.env.VITE_SOVD_BASE ?? DEFAULT_BASE;
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

function isKnownComponentId(value: string): value is EcuId {
	return BENCH_COMPONENT_IDS.includes(value as EcuId);
}

function cannedComponent(id: EcuId): SovdComponent {
	return (
		CANNED_COMPONENTS.find((component) => component.id === id) ?? {
			id,
			label: id.toUpperCase(),
			hwVersion: '--',
			swVersion: '--',
			serial: '--',
			vin: '--',
			capabilities: []
		}
	);
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
	const fallback = cannedComponent(id);
	const capabilities = capabilitiesFromEntity(entity);
	return {
		...fallback,
		id,
		label: name?.trim() || fallback.label,
		capabilities: capabilities.length > 0 ? capabilities : fallback.capabilities
	};
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

function cannedFault(componentId: EcuId, code: string): DtcEntry | undefined {
	return CANNED_DTCS.find(
		(fault) =>
			fault.component === componentId &&
			(fault.code === code || fault.id === code || fault.id === `${componentId}:${code}`)
	);
}

function mapFault(componentId: EcuId, fault: FaultResponse, detail?: FaultDetailsResponse): DtcEntry {
	const code = fault.display_code ?? fault.code ?? 'UNKNOWN';
	const fallback = cannedFault(componentId, code) ?? cannedFault(componentId, fault.code ?? code);
	const timestamp = fallback?.lastSeen ?? new Date().toISOString();
	return {
		id: `${componentId}:${fault.code ?? code}`,
		code,
		description: fault.fault_name ?? fallback?.description ?? 'Reported fault',
		severity: severityFromValue(fault.severity ?? fallback?.severity),
		status: statusFromValue(fault.status ?? fallback?.status),
		firstSeen: fallback?.firstSeen ?? timestamp,
		lastSeen: fallback?.lastSeen ?? timestamp,
		occurrences: fallback?.occurrences ?? 1,
		component: componentId,
		ecuAddress: fallback?.ecuAddress ?? 0,
		freezeFrame: freezeFrameFromEnvironment(detail?.environment_data) ?? fallback?.freezeFrame
	};
}

function executionKey(componentId: EcuId, routineId: string): string {
	return `${componentId}:${routineId}`;
}

function fallbackRoutine(componentId: EcuId, routineId: string): RoutineEntry {
	return (
		CANNED_ROUTINES.find((routine) => routine.component === componentId && routine.id === routineId) ?? {
			id: routineId,
			name: routineId,
			component: componentId,
			status: 'idle'
		}
	);
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
				: fallback.status
	};
}

export async function listComponents(): Promise<SovdComponent[]> {
	try {
		const discovered = await fetchJson<DiscoveredEntitiesResponse>('/sovd/v1/components');
		const items = discovered.items ?? [];
		const knownItems = items.filter(
			(item): item is { id: EcuId; name?: string } => !!item.id && isKnownComponentId(item.id)
		);
		if (knownItems.length === 0) {
			return CANNED_COMPONENTS;
		}
		const capabilities = await Promise.all(
			knownItems.map(async (item) => {
				try {
					return await fetchJson<EntityCapabilitiesResponse>(`/sovd/v1/components/${item.id}`);
				} catch {
					return undefined;
				}
			})
		);
		return knownItems.map((item, index) => mapComponent(item.id, item.name, capabilities[index]));
	} catch {
		return CANNED_COMPONENTS;
	}
}

export async function getComponent(id: EcuId): Promise<SovdComponent> {
	try {
		const entity = await fetchJson<EntityCapabilitiesResponse>(`/sovd/v1/components/${id}`);
		return mapComponent(id, entity.name, entity);
	} catch {
		return cannedComponent(id);
	}
}

export async function listFaults(componentId: EcuId, statusMask?: DtcStatus): Promise<DtcEntry[]> {
	try {
		const response = await fetchJson<ListOfFaultsResponse>(
			`/sovd/v1/components/${componentId}/faults?page=1&page-size=200`
		);
		const faults = (response.items ?? []).map((fault) => mapFault(componentId, fault));
		return statusMask ? faults.filter((fault) => fault.status === statusMask) : faults;
	} catch {
		const fallback = CANNED_DTCS.filter((fault) => fault.component === componentId);
		return statusMask ? fallback.filter((fault) => fault.status === statusMask) : fallback;
	}
}

export async function listAllFaults(
	componentIds: readonly EcuId[] = BENCH_COMPONENT_IDS
): Promise<DtcEntry[]> {
	const batches = await Promise.all(componentIds.map((componentId) => listFaults(componentId)));
	return batches
		.flat()
		.sort((left, right) => new Date(right.lastSeen).getTime() - new Date(left.lastSeen).getTime());
}

export async function getFaultDetail(componentId: EcuId, dtcId: string): Promise<DtcEntry> {
	const code = dtcId.includes(':') ? dtcId.split(':').pop() ?? dtcId : dtcId;
	try {
		const detail = await fetchJson<FaultDetailsResponse>(
			`/sovd/v1/components/${componentId}/faults/${encodeURIComponent(code)}`
		);
		return mapFault(componentId, detail.item ?? { code }, detail);
	} catch {
		return cannedFault(componentId, code) ?? mapFault(componentId, { code });
	}
}

export async function clearFaults(componentId: EcuId, group?: string): Promise<void> {
	const path = group
		? `/sovd/v1/components/${componentId}/faults/${encodeURIComponent(group)}`
		: `/sovd/v1/components/${componentId}/faults`;
	try {
		await fetchJson<void>(path, { method: 'DELETE' });
	} catch {
		// Demo-safe fallback: leave local UI behavior intact when the
		// backend is unavailable or the mutation route is not yet mounted.
	}
}

export async function listRoutines(componentId: EcuId): Promise<RoutineEntry[]> {
	try {
		const response = await fetchJson<OperationsListResponse>(
			`/sovd/v1/components/${componentId}/operations`
		);
		const routines = (response.items ?? []).map((operation) => {
			const fallback = fallbackRoutine(componentId, operation.id ?? 'unknown-operation');
			return {
				...fallback,
				id: operation.id ?? fallback.id,
				name: operation.name?.trim() || fallback.name,
				component: componentId
			};
		});
		return routines.length > 0
			? routines
			: CANNED_ROUTINES.filter((routine) => routine.component === componentId);
	} catch {
		return CANNED_ROUTINES.filter((routine) => routine.component === componentId);
	}
}

export async function startRoutine(componentId: EcuId, routineId: string): Promise<void> {
	try {
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
	} catch {
		// Demo-safe fallback.
	}
}

export async function stopRoutine(componentId: EcuId, routineId: string): Promise<void> {
	EXECUTION_IDS.delete(executionKey(componentId, routineId));
}

export async function pollRoutine(componentId: EcuId, routineId: string): Promise<RoutineEntry> {
	const key = executionKey(componentId, routineId);
	const executionId = EXECUTION_IDS.get(key);
	if (!executionId) {
		return fallbackRoutine(componentId, routineId);
	}
	try {
		const response = await fetchJson<ExecutionStatusResponse>(
			`/sovd/v1/components/${componentId}/operations/${encodeURIComponent(routineId)}/executions/${encodeURIComponent(executionId)}`
		);
		return {
			...fallbackRoutine(componentId, routineId),
			status: routineStatusFromExecution(response.status),
			lastResult:
				response.error && response.error.length > 0
					? response.error.map((entry) => entry.message ?? entry.title ?? 'error').join('; ')
					: undefined
		};
	} catch {
		return fallbackRoutine(componentId, routineId);
	}
}

export async function readDid(componentId: EcuId): Promise<LiveDid> {
	const fallback = cannedDid(componentId);
	try {
		const listing = await fetchJson<DatasResponse>(`/sovd/v1/components/${componentId}/data`);
		const items = (listing.items ?? []).filter(
			(item): item is DataMetadataResponse & { id: string } => typeof item.id === 'string'
		);
		if (items.length === 0) {
			return fallback;
		}
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
			vin: extractStringValue(vinValue?.data) ?? fallback.vin,
			batteryVoltage: extractNumberValue(batteryValue?.data) ?? fallback.batteryVoltage,
			temperature: extractNumberValue(temperatureValue?.data) ?? fallback.temperature,
			timestamp: new Date().toISOString()
		};
	} catch {
		return fallback;
	}
}

export async function runMlInference(componentId: EcuId): Promise<MlInferenceResult> {
	try {
		const start = await fetchJson<MlInferenceStartResponse>(mlInferencePath(componentId), {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			body: JSON.stringify({
				mode: 'single-shot',
				component_id: componentId
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
				...(isRecord(polled.result) ? polled.result : {}),
				status: polled.status ?? 'completed',
				source: 'live'
			});
		}
	} catch {
		// Demo-safe fallback when the future ML route is not mounted yet.
	}

	return cannedMlInference(componentId);
}

export async function getSession(): Promise<SessionInfo> {
	try {
		const response = await fetchJson<SessionResponse>('/sovd/v1/session');
		return {
			sessionId: response.session_id?.trim() || CANNED_SESSION.sessionId,
			level: mapSessionLevel(response.level),
			securityLevel: clampSecurityLevel(response.security_level),
			expiresAt:
				typeof response.expires_at_ms === 'number'
					? new Date(response.expires_at_ms).toISOString()
					: CANNED_SESSION.expiresAt,
			active: response.active ?? true
		};
	} catch {
		return CANNED_SESSION;
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

export async function listGatewayBackends(): Promise<GatewayBackend[]> {
	try {
		const response = await fetchJson<BackendRoutesResponse>('/sovd/v1/gateway/backends');
		const items = response.items ?? [];
		const mapped = items
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
		return mapped.length > 0 ? mapped : CANNED_BACKENDS;
	} catch {
		return CANNED_BACKENDS;
	}
}

export async function getAuditLog(limit = 50): Promise<AuditEntry[]> {
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
		return CANNED_AUDIT;
	}
}

export function telemetryPayloadToDtc(payload: unknown): DtcEntry | null {
	if (!payload || typeof payload !== 'object') {
		return null;
	}
	const record = payload as Record<string, unknown>;
	const componentId = record.component_id ?? record.componentId ?? record.component;
	const code = record.dtc ?? record.code;
	if (typeof componentId !== 'string' || typeof code !== 'string' || !isKnownComponentId(componentId)) {
		return null;
	}
	const timestamp =
		typeof record.timestamp === 'string' ? record.timestamp : new Date().toISOString();
	return {
		id: `${componentId}:${code}:${timestamp}`,
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
		component: componentId,
		ecuAddress: 0
	};
}

function cannedDid(componentId: EcuId): LiveDid {
	const volts: Record<EcuId, number> = { cvc: 14.2, sc: 12.8, bcm: 13.5 };
	const temps: Record<EcuId, number> = { cvc: 42, sc: 38, bcm: 35 };
	return {
		component: componentId,
		vin: cannedComponent(componentId).vin,
		batteryVoltage: volts[componentId],
		temperature: temps[componentId],
		timestamp: new Date().toISOString()
	};
}

export const CANNED_COMPONENTS: SovdComponent[] = [
	{
		id: 'cvc',
		label: 'CVC (Central Vehicle Controller)',
		hwVersion: 'HW-2.1.0',
		swVersion: 'SW-4.7.3',
		serial: 'CVC-001-2024',
		vin: 'WBA3A5G59ENP26705',
		capabilities: ['faults', 'operations', 'data', 'modes']
	},
	{
		id: 'sc',
		label: 'SC (Sensor Controller)',
		hwVersion: 'HW-1.5.2',
		swVersion: 'SW-3.2.1',
		serial: 'SC-002-2024',
		vin: 'WBA3A5G59ENP26705',
		capabilities: ['faults', 'data']
	},
	{
		id: 'bcm',
		label: 'BCM (Body Control Module)',
		hwVersion: 'HW-3.0.0',
		swVersion: 'SW-5.1.0',
		serial: 'BCM-003-2024',
		vin: 'WBA3A5G59ENP26705',
		capabilities: ['faults', 'operations', 'modes']
	}
];

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

export const CANNED_ROUTINES: RoutineEntry[] = [
	{ id: 'motor_self_test', name: 'Motor self test', component: 'cvc', status: 'idle' },
	{ id: 'hv_precharge', name: 'HV precharge routine', component: 'cvc', status: 'running', lastResult: 'In progress...' },
	{ id: 'safe_state_check', name: 'Safe-state supervisor check', component: 'sc', status: 'completed', lastResult: 'Pass: supervisor healthy' },
	{ id: 'relay_self_test', name: 'Relay self test', component: 'bcm', status: 'failed', lastResult: 'Error: no response' },
	{ id: 'read_vin', name: 'Read VIN', component: 'bcm', status: 'idle' }
];

export const CANNED_SESSION: SessionInfo = {
	sessionId: 'sess-9a2f3c1d',
	level: 'extended',
	securityLevel: 2,
	expiresAt: new Date(Date.now() + 120_000).toISOString(),
	active: true
};

export const CANNED_BACKENDS: GatewayBackend[] = [
	{ id: 'cvc-doip', address: '192.168.100.10:13400', protocol: 'doip', reachable: true, latencyMs: 4 },
	{ id: 'sc-uds', address: '192.168.100.11:13400', protocol: 'uds', reachable: true, latencyMs: 6 },
	{ id: 'bcm-uds', address: '192.168.100.12:13400', protocol: 'uds', reachable: false, latencyMs: 0 }
];

export const CANNED_AUDIT: AuditEntry[] = [
	{ timestamp: '2026-04-17T09:44:01Z', actor: 'tester-01', action: 'CLEAR_FAULTS', target: 'cvc', result: 'ok' },
	{ timestamp: '2026-04-17T09:43:30Z', actor: 'tester-01', action: 'START_ROUTINE', target: 'motor_self_test', result: 'ok' },
	{ timestamp: '2026-04-17T09:40:00Z', actor: 'tester-02', action: 'SESSION_ELEVATE', target: 'extended', result: 'ok' },
	{ timestamp: '2026-04-17T09:35:12Z', actor: 'tester-02', action: 'CLEAR_FAULTS', target: 'bcm', result: 'denied' },
	{ timestamp: '2026-04-17T09:10:00Z', actor: 'tester-01', action: 'SESSION_CREATE', target: 'default', result: 'ok' }
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
		status: 'completed'
	};
}
