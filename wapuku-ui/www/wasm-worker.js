
import init, {run_in_pool, init_worker, init_pool} from '../pkg/wapuku_ui';


self.onmessage = async event => {

    console.log("wapuku: worker on msg init 1: "+event.data);

     await init(undefined, event.data[1]);
     
     if (event.data[0] === "init_pool") {
         
         
         await init_pool(2);
         
         console.log("wapuku: pool init done");
         
         postMessage("done");

     } else if (event.data[0] === "init_worker") {

         postMessage("done");

         console.log("init worker done");
         
         init_worker(event.data[2]);
     }

    self.onmessage = async event => {

        console.log("wapuku: worker on msg init 2: "+event.data);
        
        switch (event.data[0]) {

            case "run_in_pool":
                console.log("run_in_pool");

                return  run_in_pool(event.data[1]);
            default:
                console.error("wapuku: no "+event.data[0]);
        }
    }
    

    // self.onmessage = async event => {
    //     console.log("working on "+event.data[0]);
    //    
    //     switch (event.data[0]) {
    //       
    //         case "run_closure":
    //             console.log("run_closure");
    //            
    //             return  run_closure(event.data[1]);
    //         default:
    //             console.error("no "+run_closure);
    //     }
    // }
}