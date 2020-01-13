/* One nested scope.
   The inner scope contains a loop.
*/

use timely::dataflow::Scope;
use timely::dataflow::operators::{Concat, ConnectLoop, Input, Inspect, Enter, Filter, Leave, Map};
use timely::order::Product;
use timely::logging::{TimelyEvent};
use timely::progress::reachability::TrackerEvent;
use timely::dataflow::operators::feedback::Feedback;

fn main() {
    let mut args = std::env::args();
    args.next();
    timely::execute_from_args(std::env::args(), move |worker| {
        worker.log_register().insert::<TimelyEvent,_>("timely", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        worker.log_register().insert::<TrackerEvent,_>("timely/tracker", |_, data|
            data.iter().for_each(|x| println!("{:?}", x.2))
        );

        // must specify types as nothing else drives inference.
        let mut input = worker.dataflow::<u64,_,_>(|child1| {
            let (input, stream) = child1.new_input::<u64>();
            child1.scoped::<Product<u64,u32>,_,_>("ScopeName", |child2| {
                let (inside_loop_handle, loop_stream) =
                    child2.feedback(Product {outer: 0, inner: 1});

                let step =
                stream
                    .enter(child2)
                    .concat(&loop_stream)
                    .map(|x| if x % 2 == 0 { x / 2 } else { 3 * x + 1 })
                    .filter(|x| x > &1);
                
                step.connect_loop(inside_loop_handle);
                step
                    .leave()
                    .inspect(move |x| println!("hello {}", x))
            });
            input
        });

        for round in 0..5 {
            input.send(round);
            input.advance_to(round);

        };

    }).unwrap();
}