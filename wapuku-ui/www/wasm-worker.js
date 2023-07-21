import {setupWorker} from "../../wapuku-common-web/www/wasm-worker-base";
import init, {run_in_pool, init_worker, init_pool} from '../pkg/wapuku_ui';


setupWorker(self, init, run_in_pool, init_worker, init_pool);