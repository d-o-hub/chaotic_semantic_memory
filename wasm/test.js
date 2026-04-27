import { fileURLToPath, pathToFileURL } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

function packageModuleUrl() {
    const packageDir = process.env.WASM_PACKAGE_DIR || __dirname;
    const modulePath = join(packageDir, 'chaotic_semantic_memory.js');
    return pathToFileURL(modulePath).href;
}

async function loadWasmBindings() {
    console.log('Loading WASM module...');

    const moduleUrl = packageModuleUrl();
    const module = await import(moduleUrl);
    const { WasmFramework, random_hypervector } = module;

    const initCandidate = [module.default, module.init, module.__wbg_init]
        .find(candidate => typeof candidate === 'function');

    if (initCandidate) {
        await initCandidate();
        console.log('WASM module initialized');
    } else {
        console.log('WASM module ready (node target auto-initializes)');
    }

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

    console.log('\nWASM smoke test passed.');
}

test().catch(err => {
    console.error('Test failed:', err);
    process.exit(1);
});
