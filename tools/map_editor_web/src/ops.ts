// Undo/redo over portal mutations.
//
// Every mutation is applied to the TMX files immediately (the backend writes
// a .bak per file on first edit); undo issues the inverse API call. Created
// portals get fresh Tiled object ids on each (re-)creation, so ops record the
// ids they produced and update them on undo/redo.

import {
  createPortal,
  ApiError,
  restorePortal,
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
      restoreToken?: string;
      label: string;
    };

export class PartialMutationError extends Error {
  constructor(message: string, public readonly original: unknown) {
    super(message);
  }
}

export async function applyOp(op: PortalOp): Promise<void> {
  switch (op.kind) {
    case "create": {
      op.createdIds = [];
      try {
        for (const req of op.requests) {
          const edge = await createPortal(req);
          op.createdIds.push({ sourceMap: edge.source, objId: edge.portal_obj_id });
        }
      } catch (error) {
        if (op.createdIds.length > 0) {
          throw new PartialMutationError(`${op.createdIds.length} portal(s) saved; the remaining create failed.`, error);
        }
        throw error;
      }
      return;
    }
    case "retarget":
      await retargetPortal(op.sourceMap, op.objId, op.after);
      return;
    case "delete":
      op.restoreToken = await deletePortal(op.edge.source, op.edge.portal_obj_id);
      return;
  }
}

export async function revertOp(op: PortalOp): Promise<void> {
  switch (op.kind) {
    case "create": {
      let removed = 0;
      try {
        for (const ref of [...op.createdIds].reverse()) {
          await deletePortal(ref.sourceMap, ref.objId);
          removed += 1;
        }
      } catch (error) {
        if (removed > 0) throw new PartialMutationError(`${removed} portal(s) deleted; undo stopped before the remaining portal(s).`, error);
        throw error;
      }
      return;
    }
    case "retarget":
      await retargetPortal(op.sourceMap, op.objId, op.before);
      return;
    case "delete": {
      if (!op.restoreToken) throw new Error("This deletion has no restoration receipt.");
      const edge = await restorePortal(op.edge.source, op.restoreToken);
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

  clear(): void {
    this.ops = [];
    this.applied = 0;
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

/** Conflicts invalidate inverse edits too; refresh once and never retry the action. */
export async function recoverMutation(
  error: unknown,
  history: OpStack,
  refresh: () => Promise<void>,
  discardDrafts: () => void,
): Promise<string> {
  const original = error instanceof PartialMutationError ? error.original : error;
  const conflict = original instanceof ApiError && original.status === 409;
  if (conflict || error instanceof PartialMutationError) {
    history.clear();
    discardDrafts();
    await refresh();
    const detail = original instanceof Error ? original.message : String(original);
    const partial = error instanceof PartialMutationError ? `${error.message} ` : "";
    return `${conflict ? "Conflict" : "Partial edit"}: ${partial}${detail} Graph refreshed; undo/redo history cleared. Review before editing again. Any earlier successful edits remain saved.`;
  }
  await refresh();
  return `Error: ${error instanceof Error ? error.message : String(error)}`;
}
