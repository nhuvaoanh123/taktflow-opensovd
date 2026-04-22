import { render, screen } from '@testing-library/svelte';
import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from 'node:child_process';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterAll, beforeAll, describe, expect, test, vi } from 'vitest';

import { runMlInference } from '../src/lib/api/sovdClient';
import UC21MlInference from '../src/lib/widgets/UC21MlInference.svelte';

async function reservePort(): Promise<number> {
	return await new Promise((resolve, reject) => {
		const server = net.createServer();
		server.once('error', reject);
		server.listen(0, '127.0.0.1', () => {
			const address = server.address();
			if (!address || typeof address === 'string') {
				server.close();
				reject(new Error('failed to reserve a TCP port'));
				return;
			}
			const { port } = address;
			server.close((error?: Error) => {
				if (error) {
					reject(error);
					return;
				}
				resolve(port);
			});
		});
	});
}

async function waitForServer(baseUrl: string, timeoutMs = 30_000): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	let lastError = 'server never responded';
	while (Date.now() < deadline) {
		try {
			const response = await fetch(`${baseUrl}/sovd/v1/components`);
			if (response.ok) {
				return;
			}
			lastError = `HTTP ${response.status}`;
		} catch (cause) {
			lastError = cause instanceof Error ? cause.message : String(cause);
		}
		await new Promise((resolve) => setTimeout(resolve, 250));
	}
	throw new Error(`timed out waiting for ${baseUrl}: ${lastError}`);
}

describe('UC21 ML inference widget', () => {
	let serverProcess: ChildProcessWithoutNullStreams | null = null;
	let baseUrl = '';

	beforeAll(async () => {
		const testDir = path.dirname(fileURLToPath(import.meta.url));
		const dashboardDir = path.resolve(testDir, '..');
		const opensovdCoreDir = path.resolve(dashboardDir, '..', 'opensovd-core');
		const build = spawnSync(
			'cargo',
			['build', '-p', 'sovd-server', '--example', 'phase8_ml_sil_server', '--quiet'],
			{
				cwd: opensovdCoreDir,
				encoding: 'utf8'
			}
		);
		if (build.status !== 0) {
			throw new Error(
				`failed to build phase8_ml_sil_server\nstdout:\n${build.stdout}\nstderr:\n${build.stderr}`
			);
		}

		const port = await reservePort();
		baseUrl = `http://127.0.0.1:${port}`;
		const binaryName =
			process.platform === 'win32' ? 'phase8_ml_sil_server.exe' : 'phase8_ml_sil_server';
		const binaryPath = path.join(
			opensovdCoreDir,
			'target',
			'debug',
			'examples',
			binaryName
		);

		serverProcess = spawn(binaryPath, [], {
			cwd: opensovdCoreDir,
			env: {
				...process.env,
				TAKTFLOW_PHASE8_ML_SERVER_ADDR: `127.0.0.1:${port}`
			},
			stdio: 'pipe'
		});

		let stderr = '';
		serverProcess.stderr.on('data', (chunk: Buffer) => {
			stderr += chunk.toString();
		});
		serverProcess.stdout.on('data', () => {
			// keep pipe drained
		});

		try {
			await waitForServer(baseUrl);
		} catch (cause) {
			serverProcess.kill();
			throw new Error(`${cause instanceof Error ? cause.message : cause}\nstderr:\n${stderr}`);
		}

		vi.stubEnv('VITE_SOVD_BASE', baseUrl);
	}, 90_000);

	afterAll(() => {
		vi.unstubAllEnvs();
		if (serverProcess) {
			serverProcess.kill();
			serverProcess = null;
		}
	});

	test('renders live inference from the standard SOVD path', async () => {
		render(UC21MlInference, { componentId: 'cvc' });

		expect(await screen.findByText('reference-fault-predictor')).toBeTruthy();
		expect(await screen.findByText('warning')).toBeTruthy();
		expect(await screen.findByText(/Predictive advisory active for CVC\./)).toBeTruthy();
		expect(await screen.findByText(/live SOVD path/)).toBeTruthy();
		expect(await screen.findByText(/\/sovd\/v1\/components\/cvc\/operations\/ml-inference\/executions/)).toBeTruthy();
	});

	test('client rollback request returns the rolled-back baseline profile', async () => {
		const rolledBack = await runMlInference('cvc', {
			action: 'rollback',
			force_trigger: 'operator_rollback'
		});
		expect(rolledBack.prediction).toBe('normal');
		expect(rolledBack.lifecycleState).toBe('rolled_back');
		expect(rolledBack.rollbackTrigger).toBe('operator_requested');
		expect(rolledBack.rollbackToModelVersion).toBe('0.9.0');
	});
});
