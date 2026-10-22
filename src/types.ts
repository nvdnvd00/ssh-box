export type HostSource = "sshconfig" | "custom";

export interface HostEntry {
  id: string;
  alias: string;
  hostname: string;
  user: string;
  port: number;
  identityFile?: string | null;
  group: string;
  source: HostSource;
}

export type PaneStatus = "connecting" | "connected" | "disconnected" | "error";

export interface LeafPaneNode {
  type: "leaf";
  paneId: string;
  kind: "ssh" | "local";
  host?: HostEntry;
  passwordOverride?: string;
  title: string;
  status: PaneStatus;
  errorMessage?: string;
  fingerprint?: string;
  reconnectNonce: number;
}

export interface SplitPaneNode {
  type: "split";
  id: string;
  direction: "horizontal" | "vertical";
  ratio: number;
  first: PaneNode;
  second: PaneNode;
}

export type PaneNode = LeafPaneNode | SplitPaneNode;

export interface WorkspaceTab {
  id: string;
  title: string;
  root: PaneNode;
  activePaneId: string;
}

export interface ProcessItem {
  pid: number;
  cpuPct: number;
  memPct: number;
  command: string;
}

export interface ServerMetrics {
  hostId: string;
  loadOne: number;
  cpuPercent: number;
  memUsedMb: number;
  memTotalMb: number;
  swapUsedMb: number;
  swapTotalMb: number;
  diskUsedGb: number;
  diskTotalGb: number;
  diskPercent: number;
  netRxKbps: number;
  netTxKbps: number;
  topProcesses: ProcessItem[];
}

export function createPaneId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `pane-${crypto.randomUUID()}`;
  }
  return `pane-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function createLocalLeaf(title = "Local"): LeafPaneNode {
  return {
    type: "leaf",
    paneId: createPaneId(),
    kind: "local",
    title,
    status: "connecting",
    reconnectNonce: 0,
  };
}

export function createSshLeaf(
  host: HostEntry,
  passwordOverride?: string,
): LeafPaneNode {
  return {
    type: "leaf",
    paneId: createPaneId(),
    kind: "ssh",
    host,
    passwordOverride,
    title: `${host.user}@${host.alias || host.hostname}`,
    status: "connecting",
    reconnectNonce: 0,
  };
}

export function findLeafById(
  node: PaneNode,
  paneId: string,
): LeafPaneNode | null {
  if (node.type === "leaf") {
    return node.paneId === paneId ? node : null;
  }
  return (
    findLeafById(node.first, paneId) ?? findLeafById(node.second, paneId)
  );
}

export function collectLeaves(node: PaneNode): LeafPaneNode[] {
  if (node.type === "leaf") {
    return [node];
  }
  return [...collectLeaves(node.first), ...collectLeaves(node.second)];
}

export function splitPaneInTree(
  node: PaneNode,
  targetPaneId: string,
  direction: "horizontal" | "vertical",
  newLeaf: LeafPaneNode,
): PaneNode {
  if (node.type === "leaf") {
    if (node.paneId !== targetPaneId) {
      return node;
    }
    return {
      type: "split",
      id: `split-${newLeaf.paneId}`,
      direction,
      ratio: 0.5,
      first: node,
      second: newLeaf,
    };
  }
  return {
    ...node,
    first: splitPaneInTree(node.first, targetPaneId, direction, newLeaf),
    second: splitPaneInTree(node.second, targetPaneId, direction, newLeaf),
  };
}

export function removePaneFromTree(
  node: PaneNode,
  targetPaneId: string,
): PaneNode | null {
  if (node.type === "leaf") {
    return node.paneId === targetPaneId ? null : node;
  }
  const nextFirst = removePaneFromTree(node.first, targetPaneId);
  const nextSecond = removePaneFromTree(node.second, targetPaneId);
  if (!nextFirst && !nextSecond) {
    return null;
  }
  if (!nextFirst) {
    return nextSecond;
  }
  if (!nextSecond) {
    return nextFirst;
  }
  return {
    ...node,
    first: nextFirst,
    second: nextSecond,
  };
}

export function updateLeafInTree(
  node: PaneNode,
  targetPaneId: string,
  updater: (leaf: LeafPaneNode) => LeafPaneNode,
): PaneNode {
  if (node.type === "leaf") {
    return node.paneId === targetPaneId ? updater(node) : node;
  }
  return {
    ...node,
    first: updateLeafInTree(node.first, targetPaneId, updater),
    second: updateLeafInTree(node.second, targetPaneId, updater),
  };
}

export function updateSplitRatioInTree(
  node: PaneNode,
  splitId: string,
  ratio: number,
): PaneNode {
  if (node.type === "leaf") {
    return node;
  }
  const clamped = Math.min(0.85, Math.max(0.15, ratio));
  if (node.id === splitId) {
    return {
      ...node,
      ratio: clamped,
    };
  }
  return {
    ...node,
    first: updateSplitRatioInTree(node.first, splitId, clamped),
    second: updateSplitRatioInTree(node.second, splitId, clamped),
  };
}
