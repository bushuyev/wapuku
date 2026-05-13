import init, {run} from '../pkg/wapuku_egui';

(async function() {
    console.log('init');

    if ('serviceWorker' in navigator) {
        const registrations = await navigator.serviceWorker.getRegistrations();
        await Promise.all(registrations.map((registration) => registration.unregister()));
    }

    await init();
    
    await run();

    console.log('init done');

})();
