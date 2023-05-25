import init, {run} from '../pkg/wapuku_ui';

(async function() {
    console.log('init');
    
    await init();
    
    await run();

    
    console.log('init done');

    // await zzz(navigator.hardwareConcurrency);

    // console.log('init thread done');
    
})();