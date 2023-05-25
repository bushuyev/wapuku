import init from '../pkg/wapuku_ui';
import {zzz} from "../pkg";

(async function run() {
    console.log('init');
    
    await init();

    
    console.log('init done');

    // await zzz(navigator.hardwareConcurrency);

    // console.log('init thread done');
    
})();