/* tslint:disable */
/* eslint-disable */
/**
* @param {any} val
* @returns {any}
*/
export function js_tags_to_lanes(val: any): any;
/**
* @param {bigint} osm_way_id
* @returns {Promise<any>}
*/
export function js_way_to_lanes(osm_way_id: bigint): Promise<any>;
/**
* @param {any} road
* @param {any} locale
* @returns {any}
*/
export function js_lanes_to_tags(road: any, locale: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly js_tags_to_lanes: (a: number, b: number) => void;
  readonly js_way_to_lanes: (a: number, b: number) => number;
  readonly js_lanes_to_tags: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h9df732ebb7c01f7c: (a: number, b: number, c: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly wasm_bindgen__convert__closures__invoke2_mut__hbe94d334d7f8f0dd: (a: number, b: number, c: number, d: number) => void;
}

/**
* Synchronously compiles the given `bytes` and instantiates the WebAssembly module.
*
* @param {BufferSource} bytes
*
* @returns {InitOutput}
*/
export function initSync(bytes: BufferSource): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
