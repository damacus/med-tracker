import init, { hydrate } from '/dashboard-hydrate-pkg.js?v=__PKG_VERSION__';

await init({ module_or_path: '/dashboard-hydrate.wasm?v=__WASM_VERSION__' });
hydrate();
