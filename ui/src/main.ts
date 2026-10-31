// SPDX-License-Identifier: MPL-2.0
// Standalone shell preview. No audio processing or private particle implementation.
import './style.css';
import './compact.css';
import './readability.css';
import {mountLayout} from './layout';
import {parameters} from './parameters';

const root = document.querySelector<HTMLDivElement>('#app')!;
const shell = mountLayout(root);
parameters(shell.controls);
const notice = document.createElement('p');
notice.className = 'renderer-notice';
notice.textContent = 'Visualization is provided by the private plugin.';
shell.viewport.append(notice);
document.querySelector('#status')!.textContent = 'UI preview';
document.querySelector('#source-kind')!.textContent = 'Output';

for (const button of root.querySelectorAll<HTMLButtonElement>('[data-view]')) {
  button.addEventListener('click', () => {
    for (const other of root.querySelectorAll('[data-view]')) other.setAttribute('aria-pressed', String(other === button));
    document.querySelector('#source-kind')!.textContent = button.dataset.view!;
  });
}
let enabled = !(window.__analysisEnabled ?? true);
function setEnabled(next: boolean) {
  if (enabled === next) return;
  enabled = next;
  shell.analysis.setAttribute('aria-pressed', String(next));
  shell.analysis.setAttribute('aria-label', `Analysis ${next ? 'On' : 'Off'}`);
  shell.analysis.querySelector('output')!.textContent = next ? 'On' : 'Off';
  document.querySelector<HTMLElement>('#paused')!.hidden = next;
  shell.canvas.hidden = !next;
  notice.hidden = !next;
  for (const button of root.querySelectorAll<HTMLButtonElement>('[data-view]')) button.disabled = !next;
}
window.setReverbAnalysisEnabled = setEnabled;
shell.analysis.addEventListener('click', () => {
  const next = !enabled;
  setEnabled(next);
  window.ipc?.postMessage(JSON.stringify({type: 'analysisEnabled', enabled: next}));
});
setEnabled(window.__analysisEnabled ?? true);
