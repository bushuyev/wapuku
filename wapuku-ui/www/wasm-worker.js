
import init, {worker_entry_point} from '../pkg-workers/wapuku_workers';


self.onmessage = async event => {

    console.log("worker init "+event.data);

    let initialised =  await  init()/*.then((r)=>{
        console.log('r='+r);
    }, (e)=>{
        console.log('e='+e);
    });*/

    console.log("worker init done");

    self.onmessage = async event => {
    //      // initialised;
    //
        console.log("event: "+event.data[0]);
        worker_entry_point(5);
    //
    //     switch (event.data[0]) {
    //         default:
    //             console.log("js  worker_entry_point");
    //             worker_entry_point(0);
    //     }
    }
}