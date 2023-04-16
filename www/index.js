import init from '../pkg/wapuku';

(async function run() {
    console.log('init');
    
    await init();
    
    console.log('init done');

})();