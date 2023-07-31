

export function setupWorker(worker, init, run_in_pool, init_worker, init_pool){

    console.log("setupWorker: init="+init);

    worker.onmessage = async event => {

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

        worker.onmessage = async event => {

            console.log("wapuku: worker on msg init 2: "+event.data);

            switch (event.data[0]) {

                case "run_in_pool":
                    console.log("run_in_pool");

                    return  run_in_pool(event.data[1]);
                default:
                    console.error("wapuku: no "+event.data[0]);
            }
        }

    }
}