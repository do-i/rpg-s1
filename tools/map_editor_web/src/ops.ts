// Undo/redo over portal mutations.
//
// Every mutation is applied to the TMX files immediately (the backend writes
// a .bak per file on first edit); undo issues the inverse API call. Created
// portals get fresh Tiled object ids on each (re-)creation, so ops record the
// ids they produced and update them on undo/redo.

import {
  createPortal,
  deletePortal,
  retargetPortal,
} from "./api";
import type {
  CreatePortalRequest,
  PortalEdgeInfo,
  RetargetPortalRequest,
} from "./api";

export interface CreatedRef {
  sourceMap: string;
  objId: number;
}

export type PortalOp =
  | {
      kind: "create";
      requests: CreatePortalRequest[];
      createdIds: CreatedRef[];
      label: string;
    }
  | {
      kind: "retarget";
      sourceMap: string;
      objId: number;
      before: RetargetPortalRequest;
      after: RetargetPortalRequest;
      label: string;
    }
  | {
      kind: "delete";
      // Everything needed to re-create the portal on undo.
      edge: PortalEdgeInfo;
      label: string;
    };

export async function applyOp(op: PortalOp): Promise<void> {
  switch (op.kind) {
    case "create": {
      const ids: CreatedRef[] = [];
      for (const req of op.requests) {
        const edge = await createPortal(req);
        ids.push({ sourceMap: edge.source, objId: edge.portal_obj_id });
      }
      op.createdIds = ids;
      return;
    }
    case "retarget":
      await retargetPortal(op.sourceMap, op.objId, op.after);
      return;
    case "delete":
      await deletePortal(op.edge.source, op.edge.portal_obj_id);
      return;
  }
}

export async function revertOp(op: PortalOp): Promise<void> {
  switch (op.kind) {
    case "create":
      for (const ref of [...op.createdIds].reverse()) {
        await deletePortal(ref.sourceMap, ref.objId);
      }
      return;
    case "retarget":
      await retargetPortal(op.sourceMap, op.objId, op.before);
      return;
    case "delete": {
      const edge = await createPortal({
        source_map: op.edge.source,
        source_rect_px: op.edge.source_rect_px,
        target_map: op.edge.target,
        target_tile: op.edge.target_tile,
      });
      // Redo must delete the portal under its new object id.
      op.edge = edge;
      return;
    }
  }
}

/** Linear undo history: ops[0..applied) are live, the rest are redoable. */
export class OpStack {
  private ops: PortalOp[] = [];
  private applied = 0;

  get canUndo(): boolean {
    return this.applied > 0;
  }

  get canRedo(): boolean {
    return this.applied < this.ops.length;
  }

  /** Record an op that has already been applied. Truncates the redo tail. */
  push(op: PortalOp): void {
    this.ops = this.ops.slice(0, this.applied);
    this.ops.push(op);
    this.applied += 1;
  }

  async undo(): Promise<string | null> {
    if (!this.canUndo) return null;
    const op = this.ops[this.applied - 1];
    await revertOp(op);
    this.applied -= 1;
    return op.label;
  }

  async redo(): Promise<string | null> {
    if (!this.canRedo) return null;
    const op = this.ops[this.applied];
    await applyOp(op);
    this.applied += 1;
    return op.label;
  }
}
