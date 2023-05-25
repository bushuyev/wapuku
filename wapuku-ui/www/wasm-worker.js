
import init from '../pkg/wapuku_ui';
import {zzz} from "../pkg";


self.onmessage = async event => {

    console.log("worker init "+event.data);

    let initialised =  await  init(undefined, event.data)/*.then((r)=>{
        console.log('r='+r);
    }, (e)=>{
        console.log('e='+e);
    });*/

    console.log("worker init done");

    self.onmessage = async event => {
    //      // initialised;
    //
        console.log("event: "+event.data[0]);
        // worker_entry_point(5);
        zzz(5);
    //
    //     switch (event.data[0]) {
    //         default:
    //             console.log("js  worker_entry_point");
    //             worker_entry_point(0);
    //     }
    }
}