import init from '../pkg/wapuku_ui';

(async function run() {
    console.log('init');
    
    await init();
    
    console.log('init done');

})();