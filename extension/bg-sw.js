import init, { background_init } from './wasm/trios_ext.js';
await init('./wasm/trios_ext_bg.wasm');
background_init();
