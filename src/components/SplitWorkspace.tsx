import React, { useRef } from "react";
import { LeafPaneNode, PaneNode, PaneStatus } from "../types";
import { TerminalPane } from "./TerminalPane";

interface SplitWorkspaceProps {
  node: PaneNode;
  activePaneId: string;
  totalPanes: number;
  onFocusPane: (paneId: string) => void;
  onUpdateLeafStatus: (
    paneId: string,
    status: PaneStatus,
    extra?: {
      errorMessage?: string;
      fingerprint?: string;
      passwordOverride?: string;
    },
  ) => void;
  onTriggerReconnect: (paneId: string, passwordOverride?: string) => void;
  onSplitPane: (
    paneId: string,
    direction: "horizontal" | "vertical",
    sourceLeaf: LeafPaneNode,
  ) => void;
  onClosePane: (paneId: string) => void;
  onResizeSplit: (splitId: string, ratio: number) => void;
}

export const SplitWorkspace: React.FC<SplitWorkspaceProps> = ({
  node,
  activePaneId,
  totalPanes,
  onFocusPane,
  onUpdateLeafStatus,
  onTriggerReconnect,
  onSplitPane,
  onClosePane,
  onResizeSplit,
}) => {
  const splitContainerRef = useRef<HTMLDivElement | null>(null);

  if (node.type === "leaf") {
    return (
      <TerminalPane
        leaf={node}
        isActive={node.paneId === activePaneId}
        showFocusBorder={totalPanes > 1}
        onFocus={() => onFocusPane(node.paneId)}
        onUpdateStatus={(status, extra) =>
          onUpdateLeafStatus(node.paneId, status, extra)
        }
        onTriggerReconnect={(pw) => onTriggerReconnect(node.paneId, pw)}
        onSplitVertical={() => onSplitPane(node.paneId, "vertical", node)}
        onSplitHorizontal={() => onSplitPane(node.paneId, "horizontal", node)}
        onClosePane={() => onClosePane(node.paneId)}
      />
    );
  }

  const isVerticalSplit = node.direction === "vertical";

  const handleMouseDownDivider = (e: React.MouseEvent) => {
    e.preventDefault();
    const container = splitContainerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (isVerticalSplit) {
        const nextRatio = (moveEvent.clientX - rect.left) / rect.width;
        onResizeSplit(node.id, nextRatio);
      } else {
        const nextRatio = (moveEvent.clientY - rect.top) / rect.height;
        onResizeSplit(node.id, nextRatio);
      }
    };

    const onMouseUp = () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  };

  const firstPct = `${(node.ratio * 100).toFixed(2)}%`;
  const secondPct = `${((1 - node.ratio) * 100).toFixed(2)}%`;

  return (
    <div
      ref={splitContainerRef}
      className={`flex h-full w-full overflow-hidden ${
        isVerticalSplit ? "flex-row" : "flex-col"
      }`}
    >
      <div
        style={{
          flexBasis: firstPct,
        }}
        className="min-w-0 min-h-0 overflow-hidden"
      >
        <SplitWorkspace
          node={node.first}
          activePaneId={activePaneId}
          totalPanes={totalPanes}
          onFocusPane={onFocusPane}
          onUpdateLeafStatus={onUpdateLeafStatus}
          onTriggerReconnect={onTriggerReconnect}
          onSplitPane={onSplitPane}
          onClosePane={onClosePane}
          onResizeSplit={onResizeSplit}
        />
      </div>

      <div
        onMouseDown={handleMouseDownDivider}
        className={`shrink-0 bg-slate-800/90 hover:bg-cyan-500/80 active:bg-cyan-400 transition-colors z-10 ${
          isVerticalSplit
            ? "w-1 cursor-col-resize"
            : "h-1 cursor-row-resize"
        }`}
      />

      <div
        style={{
          flexBasis: secondPct,
        }}
        className="min-w-0 min-h-0 overflow-hidden"
      >
        <SplitWorkspace
          node={node.second}
          activePaneId={activePaneId}
          totalPanes={totalPanes}
          onFocusPane={onFocusPane}
          onUpdateLeafStatus={onUpdateLeafStatus}
          onTriggerReconnect={onTriggerReconnect}
          onSplitPane={onSplitPane}
          onClosePane={onClosePane}
          onResizeSplit={onResizeSplit}
        />
      </div>
    </div>
  );
};
