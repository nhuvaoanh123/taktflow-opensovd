// SPDX-License-Identifier: Apache-2.0
// OpenSOVD type definitions — typed stubs; real OpenAPI codegen lands in T24.1.6

export type EcuId = 'cvc' | 'sc' | 'bcm';

export interface SovdComponent {
	id: EcuId;
	label: string;
	hwVersion: string;
	swVersion: string;
	serial: string;
	vin: string;
	capabilities: ('faults' | 'operations' | 'data' | 'modes')[];
}

export type DtcStatus =
	| 'pending'
	| 'confirmed'
	| 'suppressed'
	| 'cleared'
	| 'test_failed'
	| 'warning_indicator';

export interface DtcEntry {
	id: string;
	code: string; // e.g. "P0A1F"
	description: string;
	severity: 'low' | 'medium' | 'high' | 'critical';
	status: DtcStatus;
	firstSeen: string; // ISO 8601
	lastSeen: string;
	occurrences: number;
	component: EcuId;
	ecuAddress: number;
	freezeFrame?: Record<string, string>;
}

export type RoutineStatus = 'idle' | 'running' | 'completed' | 'failed';

export interface RoutineEntry {
	id: string;
	name: string;
	component: EcuId;
	status: RoutineStatus;
	lastResult?: string;
}

export type SessionLevel = 'default' | 'programming' | 'extended';
export type SecurityLevel = 0 | 1 | 2 | 3;

export interface SessionInfo {
	sessionId: string;
	level: SessionLevel;
	securityLevel: SecurityLevel;
	expiresAt: string; // ISO 8601
}

export interface AuditEntry {
	timestamp: string;
	actor: string;
	action: string;
	target: string;
	result: 'ok' | 'denied' | 'error';
}

export interface LiveDid {
	component: EcuId;
	vin: string;
	batteryVoltage: number; // V
	temperature: number; // °C
	timestamp: string;
}

export interface GatewayBackend {
	id: string;
	address: string;
	protocol: 'sovd' | 'uds' | 'doip';
	reachable: boolean;
	latencyMs: number;
}

export interface TelemetryFrame {
	type: 'dtc' | 'did' | 'session' | 'audit' | 'health';
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	payload: any; // raw WS frame; typed at consumer
	timestamp: string;
}
