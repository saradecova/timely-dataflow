/* Two nested scopes in sequence.
   The first inner scope contains a loop.
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
        let mut input = worker.dataflow::<u64,_,_>(|outer_scope| {
            let (input, stream) = outer_scope.new_input::<u64>();
            outer_scope.scoped::<Product<u64,u32>,_,_>("First Inner Scope", |child1| {
                
                // Create a feedback operator for First subscope.
                let (loop_instream_1, loop_outstream_1) =
                    child1.feedback(Product {outer: 0, inner: 1});
                                        
                let stream =
                stream
                    .enter(child1)  // Enter First subscope
                    .concat(&loop_outstream_1)
                    .map(|x| if x % 2 == 0 { x / 2 } else { 3 * x + 1 })
                    .filter(|x| x > &1);

                // Create Second subscope
                let stream = child1.scoped::<Product<u64,u32>,_,_>("Second Inner Scope", |child2| {
                    let (loop_instream_2, loop_outstream_2) =
                        child2.feedback(Product {outer: 0, inner: 1});

                    let stream =
                    stream
                        .enter(child2)  // Enter Second subscope
                        .concat(&loop_outstream_2)
                        .map(|x| if x % 2 == 0 { x / 2 } else { 3 * x + 1 })
                        .filter(|x| x > &1);
                
                    // Close loop of the Second inner subscope and leave subscope
                    stream.connect_loop(loop_instream_2);
                    stream.leave()
                });

                // Close loop of the First inner subscope and leave subscope
                stream.connect_loop(loop_instream_1);
                stream.leave().inspect(move |x| println!("hello {}", x))
            });
            input
        });

        for round in 0..5 {
            input.send(round);
            input.advance_to(round);
        };

    }).unwrap();
}