// SPDX-License-Identifier: MPL-2.0
// UI control transport only; no acoustic features or renderer protocol.
interface Window {
  ipc?: { postMessage(message: string): void };
  __initialParams?: number[];
  __analysisEnabled?: boolean;
  setReverbParameters?: (values: number[]) => void;
  setReverbAnalysisEnabled?: (enabled: boolean) => void;
}
