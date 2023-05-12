import init from '../pkg/wapuku_ui';
import {initThreadPool} from "../pkg";

(async function run() {
    console.log('init');
    
    await init();
    
    console.log('init done');

    await initThreadPool(navigator.hardwareConcurrency);

})();