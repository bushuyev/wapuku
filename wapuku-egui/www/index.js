import init, {run} from '../pkg/wapuku_egui';

(async function() {
    console.log('init');

    const memory = new WebAssembly.Memory({
        initial: 80,
        maximum: 50000,
        shared: true
    });
    
    await init(undefined, memory);
    
    await run();

    console.log('init done');

})();