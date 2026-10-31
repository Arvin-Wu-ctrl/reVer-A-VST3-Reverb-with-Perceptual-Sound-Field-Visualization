# reVer UI shell

The TypeScript parameter controller, markup and CSS are extracted from reVer's actual interface. The standalone entry point wires the controls and tabs for preview. It contains **no particle renderer, shaders, acoustic analysis or perceptual mapping**. The empty visualization area is intentional.

```sh
npm ci
npm run dev
```

Open the printed local URL. `npm run build` checks types and builds the preview. No audio is processed in this package, and the Analysis button alone cannot start or stop a processor without a host.

The nine parameter controls send normalized `begin` / `set` / `end` messages through an optional `window.ipc.postMessage` host bridge. A host can initialize values with `window.__initialParams`, update them with `window.setReverbParameters`, and synchronize the Analysis control with `window.setReverbAnalysisEnabled`. Markup leaves a canvas for a separately supplied renderer; no private display-frame or audio-feature implementation is included.

MPL-2.0, as stated by the repository's root LICENSE. This source addition does not change the already published VST Alpha binary or its release-specific licensing and source snapshot.
