import init, {run} from '../pkg/wapuku_egui';

(async function() {
    console.log('init');

    if ('serviceWorker' in navigator) {
        const registrations = await navigator.serviceWorker.getRegistrations();
        await Promise.all(registrations.map((registration) => registration.unregister()));
    }

    const memory = new WebAssembly.Memory({
        initial: 80,
        maximum: 50000,
        shared: true
    });
    
    await init(undefined, memory);
    
    await run();

    console.log('init done');

})();
