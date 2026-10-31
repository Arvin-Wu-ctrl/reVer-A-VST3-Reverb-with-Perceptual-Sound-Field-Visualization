// SPDX-License-Identifier: MPL-2.0
// Product markup only. Analysis, sound-to-shape mapping and rendering are excluded.
export function mountLayout(root: HTMLElement) {
  root.innerHTML = `<header><div class="brand">re<span>V</span>er<span class="brand-mark" aria-hidden="true"> /</span></div><button id="analysis-toggle" aria-pressed="true" title="Suspend signal analysis and visualization. Reverb audio stays active."><i></i><span>Analysis</span><output>On</output></button></header>
<main><div class="scene-top"><nav aria-label="Display source"><button data-view="Input" aria-pressed="false">Input</button><button data-view="Output" aria-pressed="true">Output</button><button data-view="Reverb FX" aria-pressed="false">Reverb FX</button></nav><span id="status"></span></div>
<section class="viewport"><canvas id="scene" aria-label="Visualization surface"></canvas><div id="paused" hidden>Analysis off</div><div id="scene-error" hidden>Visualization unavailable</div><div class="field-label" aria-hidden="true">L<span></span>R</div></section>
<div class="signal-strip"><span id="source-kind"></span><span id="signal"></span><span id="lead"></span></div></main><section id="controls" aria-label="Reverb controls"></section>`;
  return {
    controls: root.querySelector<HTMLElement>('#controls')!,
    viewport: root.querySelector<HTMLElement>('.viewport')!,
    canvas: root.querySelector<HTMLCanvasElement>('#scene')!,
    analysis: root.querySelector<HTMLButtonElement>('#analysis-toggle')!,
  };
}
