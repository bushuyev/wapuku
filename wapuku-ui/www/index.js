import init, {run} from '../pkg/wapuku_ui';

(async function() {
    console.log('init');

    const memory = new WebAssembly.Memory({
        initial: 80,
        maximum: 20000,
        shared: true
    });
    
    await init(undefined, memory);
    
    await run();

    console.log('init done');

})();