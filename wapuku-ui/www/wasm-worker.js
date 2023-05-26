
import init, {run_in_pool, run_closure, init_pool} from '../pkg/wapuku_ui';


self.onmessage = async event => {

    console.log("worker on msg init "+event.data);

     await init(undefined, event.data[0]);
     
     if (event.data[1] === "init_pool") {
         init_pool(2);

         console.log("pool init done");

     } else if (event.data[1] === "init_worker") {
         return  run_closure(event.data[2]);

         console.log("worker init done");
         
     }

    self.onmessage = async event => {
        switch (event.data[0]) {

            case "run_in_pool":
                console.log("run_closure");

                return  run_in_pool(event.data[1]);
            default:
                console.error("no "+run_closure);
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