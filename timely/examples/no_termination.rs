extern crate timely;

use timely::dataflow::{InputHandle};
use timely::dataflow::operators::{Concat, ConnectLoop, Filter, Input, Feedback, Map};
use timely::logging::{TimelyEvent};
use timely::progress::reachability::TrackerEvent;

fn main() {
    // initializes and runs a timely dataflow.
    let config = timely::Configuration::from_args(::std::env::args()).unwrap();
    timely::execute(config, |worker| {
        
        worker.log_register().insert::<TimelyEvent,_>("timely", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        worker.log_register().insert::<TrackerEvent,_>("timely/tracker", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );
        
        let mut input = InputHandle::new();

        worker.dataflow(|scope| {
            let stream = scope.input_from(&mut input);
            
            let (loop_handle, loop_stream) = scope.feedback(1);

            let step =
            stream
                .concat(&loop_stream)
                .map(|x| if x % 2 == 0 { x / 2 } else { x - 1 })
                .filter(|x| x > &1);

            step.connect_loop(loop_handle);
        });

        for round in 0..5 {

            input.send(round);
            input.advance_to(round);

        };

    }).unwrap();
}
