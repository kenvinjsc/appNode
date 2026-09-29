// Async heavy tasks with status + cancel, never blocking the UI.
import { useUi } from '../app/uiStore';

export async function runJob<T>(label: string, fn: (signal: AbortSignal) => Promise<T>): Promise<T | null> {
  const ctrl = new AbortController();
  const ui = useUi.getState();
  const id = ui.startJob(label, () => ctrl.abort());
  try {
    return await fn(ctrl.signal);
  } catch (e) {
    if ((e as Error).name === 'AbortError') {
      useUi.getState().toast({ kind: 'info', title: `Đã hủy: ${label}` });
      return null;
    }
    throw e;
  } finally {
    useUi.getState().endJob(id);
  }
}
