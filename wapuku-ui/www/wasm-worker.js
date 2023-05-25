
import init, {run_closure} from '../pkg/wapuku_ui';


self.onmessage = async event => {

    console.log("worker init "+event.data);

    let initialised =  await init(undefined, event.data)/*.then((r)=>{
        console.log('r='+r);
    }, (e)=>{
        console.log('e='+e);
    });*/;

    console.log("worker init done");

    self.onmessage = async event => {
        console.log("working on "+event.data[0]);
        
        switch (event.data[0]) {
            case "run_closure":
                console.log("run_closure");
                
                return  run_closure(event.data[1]);
                // return await run_closure(event.data[1]);
            default:
                console.error("no "+run_closure);
        }
    }
}