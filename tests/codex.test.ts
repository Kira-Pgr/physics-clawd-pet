import { afterEach, describe, expect, it, vi } from 'vitest';
import { AgentTracker, type CcEvent } from '../src/claude/agent';
import { ClaudeLink } from '../src/overlay/claude';
import { ccDecide, type Bus } from '../src/platform/host';
import type { Pet } from '../src/engine/pet';
import type { Bubbles, BubbleItem } from '../src/overlay/bubbles';
import type { Minis } from '../src/overlay/minis';

vi.mock('../src/platform/host', () => ({ ccDecide: vi.fn(() => Promise.resolve(true)) }));

const event = (name: string, extra: Partial<CcEvent> = {}): CcEvent => ({
  event: name, provider: 'codex', session_id: 'codex:s1', cwd: 'E:/work/demo', t: 1000, ...extra,
});

describe('Codex and Claude lifecycle integration', () => {
  it('keeps both providers active independently and labels completion', () => {
    const tracker = new AgentTracker();
    tracker.handle(event('UserPromptSubmit'));
    tracker.handle(event('PreToolUse', { provider: undefined, session_id: 's1' }));
    const done = tracker.handle(event('Stop', { t: 41000, last: 'Finished' }));
    expect(done).toContainEqual({
      kind: 'done', session: 'codex:s1', provider: 'codex', cwd: 'E:/work/demo',
      turnMs: 40000, tools: 0, last: 'Finished', window: undefined,
    });
    expect(tracker.mood(41000)).toBe('working');
    expect(tracker.list().find((s) => s.id === 'codex:s1')?.provider).toBe('codex');
    tracker.handle(event('SessionEnd'));
    expect(tracker.list().map((s) => s.id)).toEqual(['s1']);
  });

  it('interruption returns to idle and dismisses minis without celebrating or zapping', () => {
    const tracker = new AgentTracker();
    tracker.handle(event('UserPromptSubmit'));
    tracker.handle(event('SubagentStart', { agent_id: 'codex:s1:a1' }));
    const reactions = tracker.handle(event('Interrupt', { t: 2000 }));
    expect(reactions).toEqual([
      { kind: 'progress', session: 'codex:s1' },
      { kind: 'subagent-stop', id: 'codex:s1:a1' },
    ]);
    expect(tracker.mood(2000)).toBe('idle');
    expect(tracker.list()[0].subagents).toBe(0);
  });

  it('observation-only permission requests identify Codex and clear on progress', () => {
    const tracker = new AgentTracker();
    const reactions = tracker.handle(event('Notification', { notification_type: 'permission_prompt', window: 42 }));
    expect(reactions).toEqual([{
      kind: 'attention', session: 'codex:s1', provider: 'codex', cwd: 'E:/work/demo', message: '', window: 42,
    }]);
    expect(tracker.handle(event('PostToolUse', { tool_name: 'Bash', tool: 'npm test' }))).toEqual([
      { kind: 'progress', session: 'codex:s1', tool_name: 'Bash', tool: 'npm test' },
    ]);
  });
});

describe('Codex overlay reactions', () => {
  afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); });

  function overlay() {
    vi.stubGlobal('window', { setInterval: vi.fn() });
    const handlers = new Map<string, (value: any) => void>();
    const bus: Bus = { emit: (name, value) => handlers.get(name)?.(value), on: (name, handler) => { handlers.set(name, handler); } };
    const items = new Map<string, BubbleItem>();
    const pet = { cue: vi.fn(), setAgentMood: vi.fn() };
    const minis = { spawn: vi.fn(), dismiss: vi.fn() };
    const bubbles = { post: (item: BubbleItem) => items.set(item.key, item), retract: (key: string) => items.delete(key) };
    new ClaudeLink(bus, pet as unknown as Pet, bubbles as unknown as Bubbles, minis as unknown as Minis,
      () => ({ celebrate: true, doneBubble: true, zap: true, minis: true })).start();
    return { bus, items, pet, minis };
  }

  it('uses Codex names and recognizes apply_patch as a file edit', () => {
    const { bus, items } = overlay();
    bus.emit('cc-permission', { ...event('PermissionRequest'), id: 1, tool_name: 'apply_patch', tool: '*** Begin Patch' });
    expect(items.get('perm:1')?.title).toContain('Codex');
    expect(items.get('perm:1')?.title).toMatch(/修改文件|edit a file|ファイル編集/);
    expect(items.get('perm:1')?.title).not.toContain('Claude');
    expect(items.get('perm:1')?.body).toContain('Codex');
    items.get('perm:1')?.buttons[0].run();
    expect(ccDecide).toHaveBeenCalledWith(1, 'allow');
  });

  it('disabling Codex hands pending approval back and leaves Claude running', () => {
    const { bus, items, pet, minis } = overlay();
    const now = Date.now();
    bus.emit('cc-event', event('PreToolUse', { t: now, provider: undefined, session_id: 'claude:s' }));
    bus.emit('cc-event', event('SubagentStart', { t: now, agent_id: 'codex:s1:a' }));
    bus.emit('cc-permission', { ...event('PermissionRequest', { t: now }), id: 1 });
    bus.emit('cc-permission', { ...event('PermissionRequest', { t: now, provider: undefined, session_id: 'claude:s' }), id: 2 });
    bus.emit('cc-disconnect', 'codex');
    expect(ccDecide).toHaveBeenCalledWith(1, 'pass');
    expect(ccDecide).not.toHaveBeenCalledWith(2, 'pass');
    expect(items.has('perm:1')).toBe(false);
    expect(items.has('perm:2')).toBe(true);
    expect(minis.dismiss).toHaveBeenCalledWith('codex:s1:a');
    expect(pet.setAgentMood).toHaveBeenLastCalledWith('working');
  });
});
