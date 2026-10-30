export type HostSource = "sshconfig" | "custom";

export type MachineAuthMethod =
  | "passwordOrKey"
  | "identity"
  | "keyboardInteractive";

export interface HostEntry {
  id: string;
  alias: string;
  hostname: string;
  user: string;
  port: number;
  identityFile?: string | null;
  identityAgent?: string | null;
  group: string;
  source: HostSource;
  remark?: string;
  tags?: string[];
  authMethod?: MachineAuthMethod;
  identityId?: string | null;
  sshKeyId?: string | null;
  proxyProfileId?: string | null;
  jumpHostId?: string | null;
  showStatus?: boolean;
  defaultSftpPath?: string;
  selectedNetworkInterface?: string | null;
  selectedVolumeDevice?: string | null;
  agentForwarding?: boolean;
  startupCommand?: string;
  envVars?: string;
  createdDate?: number;
}

export interface MachineGroup {
  id: string;
  name: string;
  remark: string;
  createdDate: number;
}

export interface MachineTag {
  id: string;
  name: string;
  color: string;
  createdDate: number;
}

export interface IdentityItem {
  id: string;
  name: string;
  remark: string;
  userName: string;
  sshKeyId?: string | null;
  hasPassword?: boolean;
  createdDate: number;
}

export interface SshKeyItem {
  id: string;
  name: string;
  remark: string;
  keyType: string;
  publicKey: string;
  privateKeyPath: string;
  fingerprint: string;
  hasPassphrase?: boolean;
  createdDate: number;
}

export interface ConnectionProxyProfile {
  id: string;
  name: string;
  remark: string;
  proxyType: "socks5" | "httpConnect";
  host: string;
  port: number;
  authenticationMethod: "none" | "usernamePassword";
  username: string;
  hasPassword?: boolean;
  createdDate: number;
}

export interface PortForwardingRule {
  id: string;
  name: string;
  remark: string;
  mode: "local" | "remote" | "dynamic";
  machineId: string;
  localHost: string;
  localPort: number;
  remoteHost: string;
  remotePort: number;
  targetHost: string;
  targetPort: number;
  connectionProxyProfileId?: string | null;
  autoStart: boolean;
  createdDate: number;
  lastStartedDate?: number | null;
  status: "idle" | "running" | "stopped" | "failed";
  errorMessage?: string | null;
}

export interface SnippetPackage {
  id: string;
  name: string;
  remark: string;
  createdDate: number;
}

export interface SnippetItem {
  id: string;
  name: string;
  remark: string;
  code: string;
  packageId?: string | null;
  createdDate: number;
}

export interface SnippetExecutionRecord {
  id: string;
  snippetId: string;
  snippetName: string;
  machineId: string;
  machineName: string;
  executionDate: number;
  status: "finished" | "failed";
  result: string;
}

export interface ResourcesStoreData {
  groups: MachineGroup[];
  tags: MachineTag[];
  identities: IdentityItem[];
  sshKeys: SshKeyItem[];
  proxyProfiles: ConnectionProxyProfile[];
  portForwardingRules: PortForwardingRule[];
  snippetPackages: SnippetPackage[];
  snippets: SnippetItem[];
  snippetHistory: SnippetExecutionRecord[];
}

export type TabResources =
  | "machines"
  | "identities"
  | "keys"
  | "knownHosts"
  | "connectionProxyProfiles"
  | "portForwarding"
  | "snippets"
  | "docker"
  | "sftp"
  | "vpsConfig";

export type VpsStudioTab = "ssh" | "firewall" | "access" | "installer";

export type HostOs = "linux" | "macos" | "windows";
export interface InstallMethod { os: HostOs; manager: string; package: string; service: string | null }
export type ToolStatus = { kind: "notInstalled" } | { kind: "installed"; via: string[] } | { kind: "external" };
export interface ServiceState { enabled: boolean; active: boolean }
export interface ManagerState { id: string; present: boolean }
export interface ToolView {
  id: string;
  name: string;
  category: string;
  recommended: boolean;
  descriptionVi: string;
  descriptionEn: string;
  status: ToolStatus;
  service: string | null;
  serviceState: ServiceState | null;
  install: InstallMethod | null;
  blockedBy: string | null;
}
export interface InstallerScan { os: HostOs; managers: ManagerState[]; tools: ToolView[] }
export type InstallAction =
  | { kind: "install"; manager: string; package: string }
  | { kind: "uninstall"; manager: string; package: string }
  | { kind: "enableService"; service: string }
  | { kind: "installManager"; manager: string }
  | { kind: "refresh" };
export interface SearchHit { manager: string; package: string; description: string }

export type AuthUsed = { kind: "password" } | { kind: "key"; publicKeyBody: string };
export type PrivilegeLevel = "root" | "sudoNoPassword" | "sudoPassword" | "admin" | "none";
export type FirewallEngine = "ufw" | "firewalld" | "iptables" | "windows" | "none";
export type OsFamily = "linux" | "macos" | "windows";
export type SupportLevel = "full" | "readOnly" | "hidden";
export interface TabSupport { level: SupportLevel; reason: string | null }
export interface VpsSupport {
  ssh: TabSupport;
  sshRows: { port: boolean; permitRootLogin: boolean; allowDenyUsers: boolean };
  firewall: TabSupport;
  access: TabSupport;
  passwordCoupled: boolean;
}

export interface ConnectionContext {
  loginUser: string;
  auth: AuthUsed;
  clientIp: string;
  serverIp: string;
  serverPort: number;
}

export interface VpsProbe {
  privilege: PrivilegeLevel;
  context: ConnectionContext;
  osId: string;
  osVersion: string;
  opensshVersion: string;
  hasSystemd: boolean;
  sshSocketActive: boolean;
  selinuxEnforcing: boolean;
  firewallEngine: FirewallEngine;
  osFamily: OsFamily;
  support: VpsSupport;
}

export interface SshdSettings {
  port: number;
  permitRootLogin: string;
  passwordAuthentication: boolean;
  kbdInteractiveAuthentication: boolean;
  pubkeyAuthentication: boolean;
  allowUsers: string[];
  denyUsers: string[];
  maxAuthTries: number;
  maxSessions: number;
  loginGraceTime: number;
  clientAliveInterval: number;
  clientAliveCountMax: number;
}

export type WriteTarget = "dropIn" | "mainFile";

export interface SshdView {
  settings: SshdSettings;
  writeTarget: WriteTarget;
  rollbackPending: boolean;
}

export interface SshdApplyStarted {
  rollbackAt: number;
  serverNow: number;
  newPort: number | null;
}

export interface ListeningSocketItem {
  protocol: string;
  bindAddr: string;
  port: number;
  isPublic: boolean;
  processName: string;
  pid?: number | null;
  exposure: string;
}

export interface FirewallRuleItem {
  index: number;
  toPort: string;
  protocol: string;
  action: string;
  fromSource: string;
  v6: boolean;
  comment: string;
  rawSpec: string;
  effective?: boolean;
}

export interface FirewallSnapshot {
  engine: string;
  enabled: boolean;
  defaultIncoming: string;
  defaultOutgoing: string;
  ipv6Filtered: boolean;
  rules: FirewallRuleItem[];
  listeningSockets: ListeningSocketItem[];
}

export interface RemoteAuthorizedKey {
  index: number;
  keyType: string;
  publicKeyBody: string;
  comment: string;
  shortPreview: string;
  rawLine: string;
}

export interface LinuxAccountItem {
  username: string;
  uid: number;
  gid: number;
  homeDir: string;
  shell: string;
  locked: boolean;
  groups: string[];
  hasSudo: boolean;
  nopasswdSudo: boolean;
  authorizedKeys: RemoteAuthorizedKey[];
  keysFile?: string;
}

export interface AccessKeysSnapshot {
  accounts: LinuxAccountItem[];
}

export interface KnownHostEntry {
  id: string;
  hostPattern: string;
  keyType: string;
  fingerprint: string;
  publicKeyBase64: string;
  lineIndex: number;
}

export type SftpTransferMode = "fast_sync" | "chunk_stream" | "local_copy";

export interface SftpTransferProgress {
  transferId: string;
  fileName: string;
  direction: "upload" | "download" | "remote_to_remote" | "local";
  mode: SftpTransferMode;
  modeDetail: string;
  bytesTransferred: number;
  totalBytes: number;
  speedBytesPerSec: number;
  percent: number;
  status: "running" | "completed" | "failed";
  errorMessage?: string | null;
}

export interface SftpTransferResult {
  transferId: string;
  fileName: string;
  mode: SftpTransferMode;
  modeDetail: string;
  bytesTransferred: number;
  durationMs: number;
  speedBytesPerSec: number;
}

export interface SftpEntry {
  name: string;
  path: string;
  kind: "dir" | "file" | "symlink";
  size: number;
  modified: number;
  permissions: string;
  modeOctal: string;
}

export interface SftpListResponse {
  cwd: string;
  entries: SftpEntry[];
}

export type ResourceDisplayStyle = "grid" | "list";
export type DashboardDisplayStyle = "grid" | "compact" | "list";
export type ResourceSortStyle = "name" | "date";
export type DockerPanelTab =
  | "overview"
  | "containers"
  | "compose"
  | "images"
  | "volumes"
  | "networks";

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
  // Typed into the shell after every (re)connect, e.g. a docker exec
  startupCommand?: string;
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

export interface CpuCoreMetric {
  coreId: string;
  usagePct: number;
  userPct: number;
  systemPct: number;
  iowaitPct: number;
}

export interface StorageVolumeMetric {
  device: string;
  mount: string;
  totalGb: number;
  usedGb: number;
  availableGb: number;
  usedPercent: number;
  readBytesPerSec: number;
  writeBytesPerSec: number;
  iopsRead: number;
  iopsWrite: number;
}

export interface NetInterfaceMetric {
  name: string;
  ip: string;
  rxKbps: number;
  txKbps: number;
  rxPacketsPerSec: number;
  txPacketsPerSec: number;
  rxTotalBytes: number;
  txTotalBytes: number;
}

export interface ProcessItem {
  pid: number;
  cpuPct: number;
  memPct: number;
  rssKb: number;
  threads: number;
  user: string;
  command: string;
  args: string;
}

export interface GpuMetric {
  index: number;
  uuid: string;
  name: string;
  driverVersion: string;
  utilizationGpu: number;
  utilizationMem: number;
  memoryTotalMb: number;
  memoryUsedMb: number;
  memoryFreeMb: number;
  fanSpeedPct: number;
  temperatureGpu: number;
  powerDrawW: number;
  powerLimitW: number;
}

export interface DockerSummaryMetric {
  installed: boolean;
  version: string;
  totalContainers: number;
  runningContainers: number;
  pausedContainers: number;
  stoppedContainers: number;
  imagesCount: number;
}

export interface ServerMetrics {
  hostId: string;
  loadOne: number;
  loadFive: number;
  loadFifteen: number;
  cpuPercent: number;
  cpuUserPercent: number;
  cpuSystemPercent: number;
  cpuIowaitPercent: number;
  cpuModel: string;
  cpuCores: number;
  cores: CpuCoreMetric[];
  osName: string;
  hostname: string;
  uptimeSeconds: number;
  temperatureCelsius?: number | null;
  memUsedMb: number;
  memTotalMb: number;
  memFreeMb: number;
  memCachedMb: number;
  memBuffersMb: number;
  memAvailableMb: number;
  swapUsedMb: number;
  swapTotalMb: number;
  diskUsedGb: number;
  diskTotalGb: number;
  diskPercent: number;
  volumes: StorageVolumeMetric[];
  netRxKbps: number;
  netTxKbps: number;
  interfaces: NetInterfaceMetric[];
  topProcesses: ProcessItem[];
  gpus: GpuMetric[];
  docker?: DockerSummaryMetric | null;
}

export interface DockerContainerItem {
  id: string;
  name: string;
  image: string;
  state: string;
  status: string;
  ports: string;
  createdAt: string;
  command: string;
  cpuPerc: string;
  memUsage: string;
  memPerc: string;
  netIo: string;
  blockIo: string;
  pids: string;
}

export interface DockerComposeItem {
  name: string;
  status: string;
  configFiles: string;
}

export interface DockerImageItem {
  id: string;
  repository: string;
  tag: string;
  size: string;
  createdSince: string;
}

export interface DockerVolumeItem {
  name: string;
  driver: string;
  mountpoint: string;
  scope: string;
}

export interface DockerNetworkItem {
  id: string;
  name: string;
  driver: string;
  scope: string;
}

export interface DockerFullSnapshot {
  installed: boolean;
  serverVersion: string;
  os: string;
  arch: string;
  totalContainers: number;
  runningContainers: number;
  pausedContainers: number;
  stoppedContainers: number;
  totalImages: number;
  containers: DockerContainerItem[];
  composeProjects: DockerComposeItem[];
  images: DockerImageItem[];
  volumes: DockerVolumeItem[];
  networks: DockerNetworkItem[];
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
