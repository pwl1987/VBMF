/**
 * C9：agent 查询快照 → Product 资源形状。
 * `session-<hex32>` 显示形态规范化为 canonical UUID（Product `id`；
 * URL/响应统一 UUID 形态），显示串降级为 `label`。其余字段透传，
 * 新鲜度信封（generated_at_ms / observation_revision / observation_lineage）
 * 原样保留——无缓存策略下客户端唯一 stale 标注来源。
 */
import type { AgentQuerySnapshotWire } from "./types.ts";
import { wireSessionIdToUuid } from "../lib/sessionIds.ts";

export interface ProductSession {
  id: string;
  label: string;
  state: string;
  phase: string;
  outputs: string[];
  inputs: { id: string; handle: number }[];
}

export interface ProductRuntimeSnapshot {
  devices: unknown[];
  ports: unknown[];
  resources: unknown[];
  sessions: ProductSession[];
  capabilities: unknown[];
  program_switch: unknown;
  generated_at_ms: number;
  observation_revision: number;
  observation_lineage: string;
}

export function toProductRuntime(wire: AgentQuerySnapshotWire): ProductRuntimeSnapshot {
  return {
    devices: wire.devices,
    ports: wire.ports,
    resources: wire.resources,
    sessions: wire.sessions.map((s) => ({
      id: wireSessionIdToUuid(s.id),
      label: s.id,
      state: s.state,
      phase: s.phase,
      outputs: s.outputs,
      inputs: s.inputs,
    })),
    capabilities: wire.capabilities,
    program_switch: wire.program_switch,
    generated_at_ms: wire.generated_at_ms,
    observation_revision: wire.observation_revision,
    observation_lineage: wire.observation_lineage,
  };
}
