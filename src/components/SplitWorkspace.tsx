import React, { useRef } from "react";
import { LeafPaneNode, PaneNode, PaneStatus } from "../types";
import { AppLanguage, ThemeConfig } from "../themes";
import { TerminalPane } from "./TerminalPane";

interface SplitWorkspaceProps {
  node: PaneNode;
  activePaneId: string;
  totalPanes: number;
  themeConfig: ThemeConfig;
  isVisible?: boolean;
  language?: AppLanguage;
  broadcastTargetPaneIds?: string[];
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
  themeConfig,
  isVisible = true,
  language,
  broadcastTargetPaneIds,
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
        themeConfig={themeConfig}
        isVisible={isVisible}
        language={language}
        broadcastTargetPaneIds={broadcastTargetPaneIds}
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
          themeConfig={themeConfig}
          isVisible={isVisible}
          language={language}
          broadcastTargetPaneIds={broadcastTargetPaneIds}
          onFocusPane={onFocusPane}
          onUpdateLeafStatus={onUpdateLeafStatus}
          onTriggerReconnect={onTriggerReconnect}
          onSplitPane={onSplitPane}
          onClosePane={onClosePane}
          onResizeSplit={onResizeSplit}
        />
      </div>

      {/* shadcn-style ResizableHandle with animated grip indicator */}
      <div
        onMouseDown={handleMouseDownDivider}
        className={`group relative shrink-0 bg-border hover:bg-primary/70 active:bg-primary transition-colors duration-150 z-10 flex items-center justify-center ${
          isVerticalSplit
            ? "w-1 cursor-col-resize"
            : "h-1 cursor-row-resize"
        }`}
      >
        <span
          className={`rounded-full bg-muted-foreground/40 group-hover:bg-primary group-hover:scale-125 transition-all duration-150 pointer-events-none ${
            isVerticalSplit ? "w-0.5 h-5" : "h-0.5 w-5"
          }`}
        />
      </div>

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
          themeConfig={themeConfig}
          isVisible={isVisible}
          language={language}
          broadcastTargetPaneIds={broadcastTargetPaneIds}
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
