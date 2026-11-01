import assert from 'node:assert/strict';
import test from 'node:test';
import {
	createPreferences,
	migrateLegacy,
	PREFERENCES_CACHE,
	type PreferenceIo,
	type PreferenceValues
} from './preferences.ts';

function memoryStorage(entries: Record<string, string> = {}): Storage {
	const map = new Map(Object.entries(entries));
	return {
		get length() {
			return map.size;
		},
		key: (index: number) => [...map.keys()][index] ?? null,
		getItem: (key: string) => map.get(key) ?? null,
		setItem: (key: string, value: string) => void map.set(key, value),
		removeItem: (key: string) => void map.delete(key),
		clear: () => map.clear()
	};
}

function plane(initial: PreferenceValues | null = {}) {
	const state = { values: initial, saves: [] as Record<string, unknown>[], refuse: false };
	const timers: (() => void)[] = [];
	const io = (storage: Storage): PreferenceIo => ({
		storage: () => storage,
		load: async () => (state.values ? { ...state.values } : null),
		save: async (patch) => {
			if (state.refuse) return false;
			state.saves.push(patch);
			const next = { ...(state.values ?? {}) } as Record<string, unknown>;
			for (const [key, value] of Object.entries(patch)) {
				if (value === null) delete next[key];
				else next[key] = value;
			}
			state.values = next;
			return true;
		},
		schedule: (run) => {
			timers.push(run);
			return () => timers.splice(timers.indexOf(run), 1);
		}
	});
	return { state, timers, io };
}

test('old per-company widths and folds become one key per area, once', () => {
	const storage = memoryStorage({
		'restless:pane:acme:sidebar:work': '300',
		'restless:pane:reno:sidebar:work': '410',
		'restless:pane:acme:exec': '455.5',
		'restless:pane:acme:inbox': 'nonsense',
		'sidebar-fold:Teams': 'closed',
		'restless:account-rail': '{"width":280,"collapsed":true}',
		'restless:theme': 'dark'
	});
	const values = migrateLegacy(storage);
	assert.equal(values['pane.sidebar.work'], 300);
	assert.equal(values['pane.exec'], 456);
	assert.equal(values['pane.inbox'], undefined);
	assert.deepEqual(values.folds, { Teams: true });
	assert.deepEqual(values['account-rail'], { width: 280, collapsed: true });
	assert.equal(storage.getItem('restless:pane:reno:sidebar:work'), null);
	assert.equal(storage.getItem('sidebar-fold:Teams'), null);
	assert.equal(storage.getItem('restless:theme'), 'dark');

	// The store migrates on first read and never again.
	const again = memoryStorage({ 'restless:pane:acme:handover': '500' });
	const { io } = plane();
	const store = createPreferences(io(again));
	assert.equal(store.get('pane.handover'), 500);
	again.setItem('restless:pane:acme:handover', '600');
	assert.equal(createPreferences(io(again)).get('pane.handover'), 500);
});

test('a first reconcile sends up what this browser had, then the plane decides', async () => {
	const storage = memoryStorage({ 'restless:pane:acme:exec': '420' });
	const { state, io } = plane({ 'work.view': 'list' });
	const store = createPreferences(io(storage));
	await store.reconcile();
	assert.equal(store.get('work.view'), 'list');
	assert.equal(store.get('pane.exec'), 420);
	assert.deepEqual(state.saves, [{ 'pane.exec': 420 }]);

	// Reset on another device: the plane forgets it, and this browser follows.
	state.values = { 'work.view': 'list' };
	await store.reconcile();
	assert.equal(store.get('pane.exec'), undefined);
	assert.equal(JSON.parse(storage.getItem(PREFERENCES_CACHE) ?? '{}').synced, true);
});

test('a change is kept at once, sent after a pause, and survives a reconcile on its way', async () => {
	const storage = memoryStorage();
	const { state, timers, io } = plane({ 'inbox.filter': 'all' });
	const store = createPreferences(io(storage));
	let heard = 0;
	store.listen(() => (heard += 1));
	store.set('pane.inbox', 380);
	store.set('pane.inbox', 390);
	assert.equal(store.get('pane.inbox'), 390);
	assert.equal(heard, 2);
	assert.equal(timers.length, 1);
	assert.equal(state.saves.length, 0);
	assert.equal(JSON.parse(storage.getItem(PREFERENCES_CACHE) ?? '{}').values['pane.inbox'], 390);

	// The plane's answer arrives before the send: the local change stands and then goes up.
	await store.reconcile();
	assert.equal(store.get('pane.inbox'), 390);
	assert.equal(store.get('inbox.filter'), 'all');
	assert.deepEqual(state.saves, [{ 'pane.inbox': 390 }]);

	store.set('inbox.filter', null);
	await store.flush();
	assert.deepEqual(state.saves.at(-1), { 'inbox.filter': null });
	assert.equal(state.values?.['inbox.filter'], undefined);
});

test('an unreachable plane leaves this browser as it was, and the change waits', async () => {
	const storage = memoryStorage();
	const { state, io } = plane(null);
	const store = createPreferences(io(storage));
	store.set('work.sort', 'priority');
	state.refuse = true;
	await store.flush();
	await store.reconcile();
	assert.equal(store.get('work.sort'), 'priority');
	state.refuse = false;
	state.values = {};
	await store.reconcile();
	assert.deepEqual(state.saves, [{ 'work.sort': 'priority' }]);
});

test('blocked storage still keeps choices for the visit', () => {
	const store = createPreferences({
		storage: () => {
			throw new Error('blocked');
		},
		load: async () => null,
		save: async () => true,
		schedule: () => () => {}
	});
	store.set('pane.exec', 400);
	assert.equal(store.get('pane.exec'), 400);
});
