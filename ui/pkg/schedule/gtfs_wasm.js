/* @ts-self-types="./gtfs_wasm.d.ts" */

export class CachedState {
    static __wrap(ptr) {
        const obj = Object.create(CachedState.prototype);
        obj.__wbg_ptr = ptr;
        CachedStateFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        CachedStateFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_cachedstate_free(ptr, 0);
    }
}
if (Symbol.dispose) CachedState.prototype[Symbol.dispose] = CachedState.prototype.free;

/**
 * CachedState içindeki dosya istatistiklerini JSON olarak döner.
 * [{name, rows, bytes}]
 * @param {CachedState} cache
 * @returns {any}
 */
export function get_cached_file_stats(cache) {
    _assertClass(cache, CachedState);
    const ret = wasm.get_cached_file_stats(cache.__wbg_ptr);
    return ret;
}

/**
 * ZIP içindeki kök .txt dosyalarını listeler (ayrıştırma yapmadan, sadece metadata).
 * Dönen değer: JSON dizisi — [{name, uncompressed_size}]
 * @param {Uint8Array} zip_bytes
 * @returns {any}
 */
export function list_zip_files(zip_bytes) {
    const ptr0 = passArray8ToWasm0(zip_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.list_zip_files(ptr0, len0);
    return ret;
}

/**
 * K1–K5'i çalıştırır.
 * `on_stage(name, elapsed_ms)`: K1/K2/K3/K4/K5 her biri bittikten sonra çağrılır.
 * @param {Uint8Array} zip_bytes
 * @param {string} config_delta_json
 * @param {Function} on_stage
 * @returns {CachedState}
 */
export function prepare(zip_bytes, config_delta_json, on_stage) {
    const ptr0 = passArray8ToWasm0(zip_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.prepare(ptr0, len0, ptr1, len1, on_stage);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CachedState.__wrap(ret[0]);
}

/**
 * Deterministik K1–K5 hazırlığı; `today` K4 cross-reference tarihli kontrollerinde kullanılır.
 * @param {Uint8Array} zip_bytes
 * @param {string} config_delta_json
 * @param {Function} on_stage
 * @param {number} today
 * @returns {CachedState}
 */
export function prepare_with_today(zip_bytes, config_delta_json, on_stage, today) {
    const ptr0 = passArray8ToWasm0(zip_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.prepare_with_today(ptr0, len0, ptr1, len1, on_stage, today);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return CachedState.__wrap(ret[0]);
}

/**
 * Önbellekten K6+K7'yi çalıştırır.
 * `on_stage(name, elapsed_ms)`: K6 ve K7 bittikten sonra çağrılır.
 * @param {CachedState} cache
 * @param {string} config_delta_json
 * @param {Function} on_stage
 * @returns {any}
 */
export function rerun_k6_k7(cache, config_delta_json, on_stage) {
    _assertClass(cache, CachedState);
    const ptr0 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.rerun_k6_k7(cache.__wbg_ptr, ptr0, len0, on_stage);
    return ret;
}

/**
 * Deterministik K6+K7 yeniden çalıştırması; `today` takvim ve servis kontrollerinde kullanılır.
 * @param {CachedState} cache
 * @param {string} config_delta_json
 * @param {Function} on_stage
 * @param {number} today
 * @returns {any}
 */
export function rerun_k6_k7_with_today(cache, config_delta_json, on_stage, today) {
    _assertClass(cache, CachedState);
    const ptr0 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.rerun_k6_k7_with_today(cache.__wbg_ptr, ptr0, len0, on_stage, today);
    return ret;
}

/**
 * Tek bir shape'in sıralı `[[lat, lon], ...]` nokta listesini JSON döner.
 * Büyük feed modunda name_index.shape_coords peşinen serialize edilmez; UI harita
 * ikonuna tıklayınca yalnız ilgili shape'i BURADAN çeker (records bellekte canlı).
 * Bulunamazsa/geometri yoksa `[]`.
 * @param {CachedState} cache
 * @param {string} shape_id
 * @returns {any}
 */
export function shape_coords_of(cache, shape_id) {
    _assertClass(cache, CachedState);
    const ptr0 = passStringToWasm0(shape_id, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.shape_coords_of(cache.__wbg_ptr, ptr0, len0);
    return ret;
}

/**
 * Tam pipeline: K1–K7 tek seferde (config panel kullanmayan akış için).
 * `today` = tarayıcının yerel tarihi; deterministik çıktı için
 * [`validate_with_today`] kullanın.
 * @param {Uint8Array} zip_bytes
 * @param {string} config_delta_json
 * @returns {any}
 */
export function validate(zip_bytes, config_delta_json) {
    const ptr0 = passArray8ToWasm0(zip_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.validate(ptr0, len0, ptr1, len1);
    return ret;
}

/**
 * [`validate`] ile AYNI pipeline; farkı yalnız `today`'in dışarıdan verilmesi.
 *
 * Takvim/servis kuralları (CAL_*, OPR_*) "bugün"e görelidir; `validate` bunu
 * `js_sys::Date`'ten okur, dolayısıyla çıktı koşulduğu güne bağlıdır. Golden
 * baseline üretimi ve testler için tarihi sabitleyip deterministik sonuç almak
 * gerekir (native CLI'daki `--today` bayrağının karşılığı).
 *
 * `today` formatı `YYYYMMDD` (ör. 20260716). Geçersiz değer Fatal döner.
 * @param {Uint8Array} zip_bytes
 * @param {string} config_delta_json
 * @param {number} today
 * @returns {any}
 */
export function validate_with_today(zip_bytes, config_delta_json, today) {
    const ptr0 = passArray8ToWasm0(zip_bytes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(config_delta_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.validate_with_today(ptr0, len0, ptr1, len1, today);
    return ret;
}

export function wasm_init() {
    wasm.wasm_init();
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_memory_9544558992fc5400: function() {
            const ret = wasm.memory;
            return ret;
        },
        __wbg___wbindgen_number_get_f73a1244370fcc2c: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_throw_9c31b086c2b26051: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_buffer_09bfa2e33737b5fc: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_faa0a261f288f846: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_getDate_a52123c8affc9072: function(arg0) {
            const ret = arg0.getDate();
            return ret;
        },
        __wbg_getFullYear_d5d1f7de344fdc5b: function(arg0) {
            const ret = arg0.getFullYear();
            return ret;
        },
        __wbg_getMonth_de70091920053153: function(arg0) {
            const ret = arg0.getMonth();
            return ret;
        },
        __wbg_get_dcf82ab8aad1a593: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_log_eb752234eec406d1: function(arg0) {
            console.log(arg0);
        },
        __wbg_new_0_2722fcdb71a888a6: function() {
            const ret = new Date();
            return ret;
        },
        __wbg_now_81363d44c96dd239: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_timeEnd_bcacbd9f49a82f89: function(arg0, arg1) {
            console.timeEnd(getStringFromWasm0(arg0, arg1));
        },
        __wbg_time_4cbe20fc26a5a8ce: function(arg0, arg1) {
            console.time(getStringFromWasm0(arg0, arg1));
        },
        __wbg_warn_c4e0780980765a86: function(arg0) {
            console.warn(arg0);
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./gtfs_wasm_bg.js": import0,
    };
}

const CachedStateFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_cachedstate_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('gtfs_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
