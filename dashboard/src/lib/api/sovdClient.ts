// SPDX-License-Identifier: Apache-2.0
// Typed fetch wrappers for the OpenSOVD REST API.
// Real wiring targets: http://<pi-ip>:21002/sovd/v1/...
// In this stub release all calls return hardcoded canned data (T24.1.6 will
// replace with OpenAPI-generated types + real fetch).

import type {
	SovdComponent,
	DtcEntry,
	RoutineEntry,
	SessionInfo,
	LiveDid,
	GatewayBackend,
	AuditEntry
} from '$lib/types/sovd';

const BASE = typeof window !== 'undefined'
	? (import.meta.env.VITE_SOVD_BASE ?? 'http://localhost:21002')
	: 'http://localhost:21002';

async function get<T>(path: string): Promise<T> {
	// In dev/demo mode return canned data immediately
	return cannedGet<T>(path);
}

async function post<T>(path: string, body: unknown): Promise<T> {
	return cannedPost<T>(path, body);
}

// -------------------------------------------------------------------------
// Public API surface
// -------------------------------------------------------------------------

export async function listComponents(): Promise<SovdComponent[]> {
	return get<SovdComponent[]>('/sovd/v1/components');
}

export async function getComponent(id: string): Promise<SovdComponent> {
	return get<SovdComponent>(`/sovd/v1/components/${id}`);
}

export async function listFaults(componentId: string, statusMask?: string): Promise<DtcEntry[]> {
	const qs = statusMask ? `?status=${statusMask}` : '';
	return get<DtcEntry[]>(`/sovd/v1/components/${componentId}/faults${qs}`);
}

export async function getFaultDetail(componentId: string, dtcId: string): Promise<DtcEntry> {
	return get<DtcEntry>(`/sovd/v1/components/${componentId}/faults/${dtcId}`);
}

export async function clearFaults(componentId: string, group?: string): Promise<void> {
	await post<void>(`/sovd/v1/components/${componentId}/faults/clear`, { group });
}

export async function listRoutines(componentId: string): Promise<RoutineEntry[]> {
	return get<RoutineEntry[]>(`/sovd/v1/components/${componentId}/operations`);
}

export async function startRoutine(routineId: string): Promise<void> {
	await post<void>(`/sovd/v1/operations/${routineId}/start`, {});
}

export async function stopRoutine(routineId: string): Promise<void> {
	await post<void>(`/sovd/v1/operations/${routineId}/stop`, {});
}

export async function pollRoutine(routineId: string): Promise<RoutineEntry> {
	return get<RoutineEntry>(`/sovd/v1/operations/${routineId}/status`);
}

export async function readDid(componentId: string): Promise<LiveDid> {
	return get<LiveDid>(`/sovd/v1/components/${componentId}/data/live`);
}

export async function getSession(): Promise<SessionInfo> {
	return get<SessionInfo>('/sovd/v1/session');
}

export async function listGatewayBackends(): Promise<GatewayBackend[]> {
	return get<GatewayBackend[]>('/sovd/v1/gateway/backends');
}

export async function getAuditLog(limit = 50): Promise<AuditEntry[]> {
	return get<AuditEntry[]>(`/sovd/v1/audit?limit=${limit}`);
}

// -------------------------------------------------------------------------
// Canned data (T24.1.5 stub — replaced by real fetch in T24.1.6)
// -------------------------------------------------------------------------

/* eslint-disable @typescript-eslint/no-explicit-any */
function cannedGet<T>(path: string): T {
	if (path.includes('/components') && !path.includes('/faults') && !path.includes('/data') && !path.includes('/operations') && path.endsWith('/components')) {
		return CANNED_COMPONENTS as unknown as T;
	}
	if (path.includes('/faults/clear')) return undefined as unknown as T;
	if (path.includes('/faults/')) return CANNED_DTCS[0] as unknown as T;
	if (path.includes('/faults')) return CANNED_DTCS as unknown as T;
	if (path.includes('/operations')) return CANNED_ROUTINES as unknown as T;
	if (path.includes('/data/live')) return cannedDid(path) as unknown as T;
	if (path.includes('/session')) return CANNED_SESSION as unknown as T;
	if (path.includes('/gateway/backends')) return CANNED_BACKENDS as unknown as T;
	if (path.includes('/audit')) return CANNED_AUDIT as unknown as T;
	return {} as unknown as T;
}

function cannedPost<T>(_path: string, _body: unknown): T {
	return undefined as unknown as T;
}

function cannedDid(path: string): LiveDid {
	const id = path.includes('/cvc') ? 'cvc' : path.includes('/sc') ? 'sc' : 'bcm';
	const volts: Record<string, number> = { cvc: 14.2, sc: 12.8, bcm: 13.5 };
	const temps: Record<string, number> = { cvc: 42, sc: 38, bcm: 35 };
	return {
		component: id as any,
		vin: 'WBA3A5G59ENP26705',
		batteryVoltage: volts[id],
		temperature: temps[id],
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
		id: 'dtc-001', code: 'P0A1F', description: 'High Voltage Battery Pack Voltage Sense Circuit',
		severity: 'critical', status: 'confirmed', component: 'cvc', ecuAddress: 0x7E0,
		firstSeen: '2026-04-15T08:22:11Z', lastSeen: '2026-04-17T09:44:02Z', occurrences: 7,
		freezeFrame: { 'Battery Voltage': '3.2V', 'Temperature': '45°C', 'SOC': '12%' }
	},
	{
		id: 'dtc-002', code: 'C0035', description: 'Left Front Wheel Speed Sensor Circuit',
		severity: 'medium', status: 'pending', component: 'cvc', ecuAddress: 0x7E0,
		firstSeen: '2026-04-16T14:10:00Z', lastSeen: '2026-04-17T09:44:02Z', occurrences: 3
	},
	{
		id: 'dtc-003', code: 'U0100', description: 'Lost Communication With ECM/PCM A',
		severity: 'high', status: 'confirmed', component: 'sc', ecuAddress: 0x7E4,
		firstSeen: '2026-04-14T12:00:00Z', lastSeen: '2026-04-17T07:30:00Z', occurrences: 12
	},
	{
		id: 'dtc-004', code: 'B1234', description: 'Driver Door Ajar Switch Circuit Open',
		severity: 'low', status: 'cleared', component: 'bcm', ecuAddress: 0x726,
		firstSeen: '2026-04-10T10:00:00Z', lastSeen: '2026-04-11T11:00:00Z', occurrences: 1
	},
	{
		id: 'dtc-005', code: 'P0D00', description: 'Electric Vehicle Battery Pack Malfunction',
		severity: 'critical', status: 'test_failed', component: 'cvc', ecuAddress: 0x7E0,
		firstSeen: '2026-04-17T08:00:00Z', lastSeen: '2026-04-17T09:44:02Z', occurrences: 2
	},
	{
		id: 'dtc-006', code: 'C0051', description: 'Steering Position Sensor Circuit',
		severity: 'medium', status: 'suppressed', component: 'sc', ecuAddress: 0x7E4,
		firstSeen: '2026-04-12T09:15:00Z', lastSeen: '2026-04-12T09:15:00Z', occurrences: 1
	}
];

export const CANNED_ROUTINES: RoutineEntry[] = [
	{ id: 'rt-001', name: 'Battery Capacity Test', component: 'cvc', status: 'idle' },
	{ id: 'rt-002', name: 'Wheel Speed Calibration', component: 'cvc', status: 'running', lastResult: 'In progress…' },
	{ id: 'rt-003', name: 'Sensor Offset Learn', component: 'sc', status: 'completed', lastResult: 'Pass: offset=+0.02V' },
	{ id: 'rt-004', name: 'Door Lock Actuator Test', component: 'bcm', status: 'failed', lastResult: 'Error: no response' },
	{ id: 'rt-005', name: 'HVAC Self-Check', component: 'bcm', status: 'idle' }
];

export const CANNED_SESSION: SessionInfo = {
	sessionId: 'sess-9a2f3c1d',
	level: 'extended',
	securityLevel: 2,
	expiresAt: new Date(Date.now() + 120_000).toISOString()
};

export const CANNED_BACKENDS: GatewayBackend[] = [
	{ id: 'cvc-doip', address: '192.168.100.10:13400', protocol: 'doip', reachable: true, latencyMs: 4 },
	{ id: 'sc-uds', address: '192.168.100.11:13400', protocol: 'uds', reachable: true, latencyMs: 6 },
	{ id: 'bcm-uds', address: '192.168.100.12:13400', protocol: 'uds', reachable: false, latencyMs: 0 }
];

export const CANNED_AUDIT: AuditEntry[] = [
	{ timestamp: '2026-04-17T09:44:01Z', actor: 'tester-01', action: 'CLEAR_FAULTS', target: 'cvc', result: 'ok' },
	{ timestamp: '2026-04-17T09:43:30Z', actor: 'tester-01', action: 'START_ROUTINE', target: 'rt-002', result: 'ok' },
	{ timestamp: '2026-04-17T09:40:00Z', actor: 'tester-02', action: 'SESSION_ELEVATE', target: 'extended', result: 'ok' },
	{ timestamp: '2026-04-17T09:35:12Z', actor: 'tester-02', action: 'CLEAR_FAULTS', target: 'bcm', result: 'denied' },
	{ timestamp: '2026-04-17T09:10:00Z', actor: 'tester-01', action: 'SESSION_CREATE', target: 'default', result: 'ok' }
];
/* eslint-enable @typescript-eslint/no-explicit-any */
