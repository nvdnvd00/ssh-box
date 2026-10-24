import React, { useMemo, useRef } from "react";
import { LeafPaneNode, PaneNode, PaneStatus } from "../types";
import { AppLanguage, ThemeConfig } from "../themes";
import { TerminalPane } from "./TerminalPane";

interface PaneRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface PositionedLeaf extends PaneRect {
  leaf: LeafPaneNode;
}

interface PositionedDivider {
  id: string;
  direction: "horizontal" | "vertical";
  x: number;
  y: number;
  length: number;
  parentStart: number;
  parentSize: number;
}

function computeSplitLayout(
  node: PaneNode,
  rect: PaneRect = { x: 0, y: 0, w: 1, h: 1 },
): { leaves: PositionedLeaf[]; dividers: PositionedDivider[] } {
  if (node.type === "leaf") {
    return { leaves: [{ leaf: node, ...rect }], dividers: [] };
  }

  if (node.direction === "vertical") {
    const firstW = rect.w * node.ratio;
    const splitX = rect.x + firstW;
    const first = computeSplitLayout(node.first, {
      x: rect.x,
      y: rect.y,
      w: firstW,
      h: rect.h,
    });
    const second = computeSplitLayout(node.second, {
      x: splitX,
      y: rect.y,
      w: rect.w - firstW,
      h: rect.h,
    });
    return {
      leaves: [...first.leaves, ...second.leaves],
      dividers: [
        ...first.dividers,
        {
          id: node.id,
          direction: "vertical",
          x: splitX,
          y: rect.y,
          length: rect.h,
          parentStart: rect.x,
          parentSize: rect.w,
        },
        ...second.dividers,
      ],
    };
  }

  const firstH = rect.h * node.ratio;
  const splitY = rect.y + firstH;
  const first = computeSplitLayout(node.first, {
    x: rect.x,
    y: rect.y,
    w: rect.w,
    h: firstH,
  });
  const second = computeSplitLayout(node.second, {
    x: rect.x,
    y: splitY,
    w: rect.w,
    h: rect.h - firstH,
  });
  return {
    leaves: [...first.leaves, ...second.leaves],
    dividers: [
      ...first.dividers,
      {
        id: node.id,
        direction: "horizontal",
        x: rect.x,
        y: splitY,
        length: rect.w,
        parentStart: rect.y,
        parentSize: rect.h,
      },
      ...second.dividers,
    ],
  };
}

interface SplitWorkspaceProps {
  node: PaneNode;
  activePaneId: string;
  totalPanes: number;
  themeConfig: ThemeConfig;
  isVisible?: boolean;
  language?: AppLanguage;
  copyOnSelect?: boolean;
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
  copyOnSelect,
  broadcastTargetPaneIds,
  onFocusPane,
  onUpdateLeafStatus,
  onTriggerReconnect,
  onSplitPane,
  onClosePane,
  onResizeSplit,
}) => {
  const splitContainerRef = useRef<HTMLDivElement | null>(null);
  const { leaves, dividers } = useMemo(() => {
    const layout = computeSplitLayout(node);
    layout.leaves.sort((a, b) => a.leaf.paneId.localeCompare(b.leaf.paneId));
    return layout;
  }, [node]);

  const handleMouseDownDivider = (
    e: React.MouseEvent,
    divider: PositionedDivider,
  ) => {
    e.preventDefault();
    const container = splitContainerRef.current;
    if (!container || divider.parentSize <= 0) return;
    const rect = container.getBoundingClientRect();

    const onMouseMove = (moveEvent: MouseEvent) => {
      if (divider.direction === "vertical") {
        const relX = (moveEvent.clientX - rect.left) / rect.width;
        onResizeSplit(
          divider.id,
          (relX - divider.parentStart) / divider.parentSize,
        );
      } else {
        const relY = (moveEvent.clientY - rect.top) / rect.height;
        onResizeSplit(
          divider.id,
          (relY - divider.parentStart) / divider.parentSize,
        );
      }
    };

    const onMouseUp = () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  };

  return (
    <div
      ref={splitContainerRef}
      className="relative h-full w-full overflow-hidden"
    >
      {leaves.map(({ leaf, x, y, w, h }) => (
        <div
          key={leaf.paneId}
          style={{
            left: `${(x * 100).toFixed(4)}%`,
            top: `${(y * 100).toFixed(4)}%`,
            width: `${(w * 100).toFixed(4)}%`,
            height: `${(h * 100).toFixed(4)}%`,
          }}
          className="absolute min-w-0 min-h-0 overflow-hidden"
        >
          <TerminalPane
            leaf={leaf}
            isActive={leaf.paneId === activePaneId}
            showFocusBorder={totalPanes > 1}
            themeConfig={themeConfig}
            isVisible={isVisible}
            language={language}
            copyOnSelect={copyOnSelect}
            broadcastTargetPaneIds={broadcastTargetPaneIds}
            onFocus={() => onFocusPane(leaf.paneId)}
            onUpdateStatus={(status, extra) =>
              onUpdateLeafStatus(leaf.paneId, status, extra)
            }
            onTriggerReconnect={(pw) => onTriggerReconnect(leaf.paneId, pw)}
            onSplitVertical={() => onSplitPane(leaf.paneId, "vertical", leaf)}
            onSplitHorizontal={() =>
              onSplitPane(leaf.paneId, "horizontal", leaf)
            }
            onClosePane={() => onClosePane(leaf.paneId)}
          />
        </div>
      ))}

      {dividers.map((div) => {
        const isVertical = div.direction === "vertical";
        return (
          <div
            key={div.id}
            onMouseDown={(e) => handleMouseDownDivider(e, div)}
            style={
              isVertical
                ? {
                    left: `calc(${(div.x * 100).toFixed(4)}% - 2px)`,
                    top: `${(div.y * 100).toFixed(4)}%`,
                    height: `${(div.length * 100).toFixed(4)}%`,
                  }
                : {
                    left: `${(div.x * 100).toFixed(4)}%`,
                    top: `calc(${(div.y * 100).toFixed(4)}% - 2px)`,
                    width: `${(div.length * 100).toFixed(4)}%`,
                  }
            }
            className={`absolute group shrink-0 bg-border hover:bg-primary/70 active:bg-primary transition-colors duration-150 z-10 flex items-center justify-center ${
              isVertical ? "w-1 cursor-col-resize" : "h-1 cursor-row-resize"
            }`}
          >
            <span
              className={`rounded-full bg-muted-foreground/40 group-hover:bg-primary group-hover:scale-125 transition-all duration-150 pointer-events-none ${
                isVertical ? "w-0.5 h-5" : "h-0.5 w-5"
              }`}
            />
          </div>
        );
      })}
    </div>
  );
};
