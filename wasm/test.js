import { fileURLToPath, pathToFileURL } from 'url';
import { dirname, join } from 'path';
import { existsSync, readFileSync } from 'fs';

// The wasm path is package-relative by construction (`packageDir` is
// WASM_PACKAGE_DIR or this file's directory); ESLint's security plugin cannot
// see that and flags the two non-literal fs calls below.
/* eslint-disable security/detect-non-literal-fs-filename */

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function packageDirPath() {
    return process.env.WASM_PACKAGE_DIR || __dirname;
}

function packageModuleUrl() {
    const modulePath = join(packageDirPath(), 'chaotic_semantic_memory.js');
    return pathToFileURL(modulePath).href;
}

async function loadWasmBindings() {
    console.log('Loading WASM module...');

    const packageDir = packageDirPath();
    const moduleUrl = packageModuleUrl();
    const module = await import(moduleUrl);
    // --target nodejs produces CJS whose `module.exports` lands on `default`;
    // --target web produces ESM where the bindings are named exports and
    // `default` is the init function. Merge both views so one smoke test can
    // run against either target — including the release/web artifact CI now
    // builds and the release publishes.
    const defaultExport = module.default;
    const bindings =
        defaultExport && typeof defaultExport === 'object'
            ? { ...module, ...defaultExport }
            : { ...module };

    const initCandidate = [module.default, module.init, module.__wbg_init]
        .find(candidate => typeof candidate === 'function');

    if (initCandidate) {
        // `--target web` glue resolves the .wasm with `fetch(import.meta.url)`,
        // and Node's fetch rejects file:// URLs. Hand the bytes to init instead
        // so the smoke test can run against the shipped (release/web) package.
        const wasmPath = join(packageDir, 'chaotic_semantic_memory_bg.wasm');
        if (existsSync(wasmPath)) {
            await initCandidate({ module_or_path: readFileSync(wasmPath) });
            console.log('WASM module initialized from', wasmPath);
        } else {
            await initCandidate();
            console.log('WASM module initialized');
        }
    } else {
        console.log('WASM module ready (node target auto-initializes)');
    }

    const { WasmFramework, random_hypervector } = bindings;

    if (!WasmFramework || !random_hypervector) {
        throw new Error('WASM package missing expected exports');
    }

    return { WasmFramework, random_hypervector };
}

async function test() {
    const { WasmFramework, random_hypervector } = await loadWasmBindings();

    console.log('\nTesting hypervector utilities...');
    const vecA = random_hypervector();
    const vecB = random_hypervector();
    if (vecA.length !== vecB.length) {
        throw new Error('Hypervectors should have equal length');
    }

    console.log('\nTesting WasmFramework APIs...');
    const framework = await WasmFramework.new();
    console.log('Framework ready');

    console.log('Injecting cat concept...');
    await framework.inject_concept('cat', vecA).catch(err => {
        throw new Error(`Failed to inject cat concept: ${err}`);
    });
    console.log('Injecting dog concept...');
    await framework.inject_concept('dog', vecB).catch(err => {
        throw new Error(`Failed to inject dog concept: ${err}`);
    });
    console.log('Associating cat -> dog...');
    await framework.associate('cat', 'dog', 0.8).catch(err => {
        throw new Error(`Failed to associate cat -> dog: ${err}`);
    });
    console.log('Injected cat/dog concepts and association');

    console.log('Running probe...');
    const probeHits = await framework.probe(vecA, 5);
    if (probeHits.length === 0) {
        throw new Error('Probe returned no hits');
    }
    console.log('Probe hits:', probeHits.slice(0, 3));

    console.log('Fetching associations...');
    const associations = await framework.get_associations('cat');
    if (!associations.some(entry => entry.to === 'dog')) {
        throw new Error('Association cat -> dog missing');
    }

    console.log('Retrieving metrics snapshot...');
    const metrics = await framework.metrics_snapshot();
    if (!metrics || metrics.concepts_injected_total < 2) {
        throw new Error('Metrics snapshot missing injected concept counts');
    }

    const exported = await framework.exportToBytes();
    if (exported.length === 0) {
        throw new Error('Export should produce non-zero bytes');
    }
    console.log(`Exported ${exported.length} bytes`);

    console.log('Testing importFromBytes round trip...');
    const framework2 = await WasmFramework.new();
    const importedCount = await framework2.importFromBytes(exported, false);
    if (importedCount !== 2) {
        throw new Error(`Expected 2 imported concepts, got ${importedCount}`);
    }
    const catAssoc = await framework2.get_associations('cat');
    if (!catAssoc.some(entry => entry.to === 'dog')) {
        throw new Error('Imported association cat -> dog missing');
    }
    console.log(`Successfully imported ${importedCount} concepts and verified associations`);

    console.log('\nWASM smoke test passed.');
}

test().catch(err => {
    console.error('Test failed:', err);
    process.exit(1);
});
